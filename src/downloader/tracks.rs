use super::Downloader;
use crate::archive::filetime_from_http_date;
use crate::metadata::extract_year_from_date;
use crate::types::{DiscInfo, Track};
use anyhow::Result;
use filetime::set_file_times;
use id3::TagLike;
use std::path::Path;
use tracing::{debug, info, warn};

impl Downloader {
    pub(super) async fn download_tracks_for_format(
        &self,
        disc_info: &DiscInfo,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        if format == "FLAC" {
            warn!(
                "FLAC 格式暂不支持通过 API 下载，请前往 https://www.dizzylab.net/d/{}/ 手动下载 - {}",
                disc_info.id, disc_info.title
            );
            return Ok(());
        }

        if disc_info.tracks.is_empty() {
            warn!("专辑 {} 没有曲目信息，跳过格式 {}", disc_info.title, format);
            return Ok(());
        }

        let target_dir = album_dir.to_path_buf();
        std::fs::create_dir_all(&target_dir)?;

        let ext = format_to_extension(format);
        info!(
            "下载格式 {} ({} 首曲目) - {}",
            format,
            disc_info.tracks.len(),
            disc_info.title
        );

        for (idx, track) in disc_info.tracks.iter().enumerate() {
            let track_num = idx + 1;
            let file_name = format!(
                "{} {}.{}",
                track_num,
                self.sanitize_filename(&track.title),
                ext
            );
            let file_path = target_dir.join(&file_name);

            // Get CDN URL — needed for download and for HEAD check.
            let cdn_url = match self
                .client
                .get_track_download_url(&disc_info.id, &track.id, format, &self.token)
                .await
            {
                Ok(url) => url,
                Err(e) => {
                    warn!("获取曲目 {} 下载链接失败: {}", track.title, e);
                    continue;
                }
            };

            // Determine whether to skip download (size-based, ±20%).
            let mut needs_download = true;
            let mut head_last_modified: Option<String> = None;

            if self.config.behavior.skip_existing && file_path.exists() {
                match self.client.head_url(&cdn_url).await {
                    Ok(meta) => {
                        head_last_modified = meta.last_modified.clone();
                        let local_size = file_path.metadata().map(|m| m.len()).unwrap_or(0);
                        let should_skip = if let Some(remote_size) = meta.content_length {
                            size_within_tolerance(local_size, remote_size, 0.20)
                        } else {
                            // No Content-Length: skip if file is non-empty.
                            local_size > 0
                        };
                        if should_skip {
                            debug!("文件大小接近，跳过下载: {}", file_name);
                            needs_download = false;
                        } else {
                            info!("文件大小差异过大，重新下载: {}", file_name);
                        }
                    }
                    Err(e) => {
                        warn!("HEAD 请求失败，重新下载 {}: {}", file_name, e);
                    }
                }
            }

            // Download if needed.
            let last_modified = if needs_download {
                match self.client.stream_to_file(&cdn_url, &file_path).await {
                    Ok(lm) => lm,
                    Err(e) => {
                        warn!("下载曲目 {} 失败: {}", track.title, e);
                        continue;
                    }
                }
            } else {
                head_last_modified
            };

            // Always write ID3 tags (whether freshly downloaded or skipped).
            let cover_path = cover_path_for_disc(disc_info, album_dir);
            if let Err(e) = write_mp3_tags(
                &file_path,
                disc_info,
                track,
                track_num as u32,
                disc_info.tracks.len() as u32,
                &cover_path,
            ) {
                warn!("写入ID3标签失败 {}: {}", file_name, e);
            } else {
                debug!("已写入ID3标签: {}", file_name);
            }

            // Set timestamp AFTER tag write, since tag write resets mtime.
            if let Some(date_str) = &last_modified {
                if let Some(ft) = filetime_from_http_date(date_str) {
                    if let Err(e) = set_file_times(&file_path, ft, ft) {
                        warn!("设置文件时间戳失败 {}: {}", file_name, e);
                    }
                }
            }

            debug!("已保存: {}", file_name);
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(())
    }
}

fn format_to_extension(format: &str) -> &str {
    match format {
        "128" | "320" => "mp3",
        "FLAC" => "flac",
        _ => "bin",
    }
}

/// Parse a Dizzylab release_date string (e.g. "2023年4月1日" or "2023-04-01") into an id3::Timestamp.
fn parse_release_date(date_str: &str) -> Option<id3::Timestamp> {
    // Chinese format: "2023年4月1日"
    if let Some(caps) = regex::Regex::new(r"(\d{4})年(\d{1,2})月(\d{1,2})日")
        .unwrap()
        .captures(date_str)
    {
        let year = caps.get(1)?.as_str().parse::<i32>().ok()?;
        let month = caps.get(2)?.as_str().parse::<u8>().ok()?;
        let day = caps.get(3)?.as_str().parse::<u8>().ok()?;
        return Some(id3::Timestamp {
            year,
            month: Some(month),
            day: Some(day),
            hour: None,
            minute: None,
            second: None,
        });
    }
    // ISO format: "2023-04-01" or "2023/04/01"
    if let Some(caps) = regex::Regex::new(r"(\d{4})[/-](\d{1,2})[/-](\d{1,2})")
        .unwrap()
        .captures(date_str)
    {
        let year = caps.get(1)?.as_str().parse::<i32>().ok()?;
        let month = caps.get(2)?.as_str().parse::<u8>().ok()?;
        let day = caps.get(3)?.as_str().parse::<u8>().ok()?;
        return Some(id3::Timestamp {
            year,
            month: Some(month),
            day: Some(day),
            hour: None,
            minute: None,
            second: None,
        });
    }
    None
}

fn size_within_tolerance(local: u64, remote: u64, tolerance: f64) -> bool {
    if remote == 0 {
        return local == 0;
    }
    let diff = (local as f64 - remote as f64).abs();
    diff / remote as f64 <= tolerance
}

fn cover_path_for_disc(disc_info: &DiscInfo, album_dir: &Path) -> std::path::PathBuf {
    let ext = if disc_info.cover.contains(".png") {
        "png"
    } else if disc_info.cover.contains(".webp") {
        "webp"
    } else {
        "jpg"
    };
    album_dir.join(format!("cover.{ext}"))
}

fn write_mp3_tags(
    file_path: &Path,
    disc_info: &DiscInfo,
    track: &Track,
    track_num: u32,
    total_tracks: u32,
    cover_path: &Path,
) -> Result<()> {
    let mut tag = id3::Tag::read_from_path(file_path).unwrap_or_else(|_| id3::Tag::new());

    tag.set_title(&track.title);
    tag.set_album(&disc_info.title);

    let artist = if track.authers.is_empty() {
        &disc_info.label
    } else {
        &track.authers
    };
    tag.set_artist(artist);
    tag.set_album_artist(&disc_info.label);

    tag.set_track(track_num);
    tag.set_total_tracks(total_tracks);

    // Year (TYER compat) + full date as TDRC (ID3v2.4 recording time).
    if let Some(date_str) = disc_info.release_date.as_deref() {
        if let Some(year_str) = extract_year_from_date(date_str) {
            if let Ok(year) = year_str.parse::<i32>() {
                tag.set_year(year);
            }
        }
        if let Some(ts) = parse_release_date(date_str) {
            tag.set_date_recorded(ts);
        }
    }

    // All genre tags joined with null separator (ID3v2.4 multi-value text frame).
    if !disc_info.tags.is_empty() {
        tag.set_genre(disc_info.tags.join("\0"));
    }

    // Dizzylab disc ID as custom TXXX frame.
    tag.add_frame(id3::frame::ExtendedText {
        description: "DIZZYLAB_ID".to_string(),
        value: disc_info.id.clone(),
    });

    if cover_path.exists() {
        if let Ok(data) = std::fs::read(cover_path) {
            let mime_type = if cover_path.extension().and_then(|e| e.to_str()) == Some("png") {
                "image/png".to_string()
            } else {
                "image/jpeg".to_string()
            };
            // Remove any existing cover frames before adding.
            tag.remove_picture_by_type(id3::frame::PictureType::CoverFront);
            tag.add_frame(id3::frame::Picture {
                mime_type,
                picture_type: id3::frame::PictureType::CoverFront,
                description: String::new(),
                data,
            });
        }
    }

    tag.write_to_path(file_path, id3::Version::Id3v24)?;
    Ok(())
}
