use super::Downloader;
use crate::archive::filetime_from_http_date;
use crate::metadata::{extract_year_from_date, normalize_date};
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

            let cover_path = cover_path_for_disc(disc_info, album_dir);

            if self.config.behavior.skip_existing && file_path.exists() {
                if file_has_dizzylab_tag(&file_path, &disc_info.id, format) {
                    debug!("已有完整标签，跳过: {}", file_name);
                    continue;
                }
                // File exists but lacks our tags (e.g. old download) — re-tag only.
                debug!("文件已存在但缺少标签，补写标签: {}", file_name);
                let saved_mtime = std::fs::metadata(&file_path)
                    .ok()
                    .map(|m| filetime::FileTime::from_last_modification_time(&m));
                if let Err(e) = write_mp3_tags(
                    &file_path,
                    disc_info,
                    track,
                    track_num as u32,
                    disc_info.tracks.len() as u32,
                    &cover_path,
                    format,
                ) {
                    warn!("写入ID3标签失败 {}: {}", file_name, e);
                } else if let Some(ft) = saved_mtime {
                    if let Err(e) = set_file_times(&file_path, ft, ft) {
                        warn!("还原MP3时间戳失败 {}: {}", file_name, e);
                    }
                }
                continue;
            }

            // File does not exist — fetch CDN URL and download.
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

            let last_modified = match self.client.stream_to_file(&cdn_url, &file_path).await {
                Ok(lm) => lm,
                Err(e) => {
                    warn!("下载曲目 {} 失败: {}", track.title, e);
                    continue;
                }
            };

            if let Err(e) = write_mp3_tags(
                &file_path,
                disc_info,
                track,
                track_num as u32,
                disc_info.tracks.len() as u32,
                &cover_path,
                format,
            ) {
                warn!("写入ID3标签失败 {}: {}", file_name, e);
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

/// Parse a Dizzylab release_date string into an id3::Timestamp.
/// Delegates normalization to `metadata::normalize_date` (→ "YYYY-MM-DD").
fn parse_release_date(date_str: &str) -> Option<id3::Timestamp> {
    let iso = normalize_date(date_str);
    let mut parts = iso.splitn(3, '-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next().and_then(|s| s.parse::<u8>().ok());
    let day = parts.next().and_then(|s| s.parse::<u8>().ok());
    Some(id3::Timestamp {
        year,
        month,
        day,
        hour: None,
        minute: None,
        second: None,
    })
}

fn file_has_dizzylab_tag(file_path: &Path, disc_id: &str, format: &str) -> bool {
    let Ok(tag) = id3::Tag::read_from_path(file_path) else {
        return false;
    };
    let mut has_id = false;
    let mut bitrate_ok = true; // pass if BITRATE frame absent (old file)
    for t in tag.extended_texts() {
        if t.description == "DIZZYLAB_ID" {
            has_id = t.value == disc_id;
        }
        if t.description == "BITRATE" {
            bitrate_ok = t.value == format;
        }
    }
    has_id && bitrate_ok
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

// ── FLAC / Vorbis Comment helpers ────────────────────────────────────────────

impl Downloader {
    /// Write Vorbis Comments (and optional cover art) to every `.flac` file in
    /// `album_dir`, matching them to `disc_info.tracks` by sort-order index.
    /// Files that already carry a valid `DIZZYLAB_ID` tag are skipped when
    /// `skip_existing` is enabled.
    pub(super) fn tag_flac_files(&self, disc_info: &DiscInfo, album_dir: &Path) {
        if disc_info.tracks.is_empty() {
            return;
        }

        let mut flac_files: Vec<_> = match std::fs::read_dir(album_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.eq_ignore_ascii_case("flac"))
                        .unwrap_or(false)
                })
                .map(|e| e.path())
                .collect(),
            Err(_) => return,
        };
        flac_files.sort();

        if flac_files.is_empty() {
            return;
        }

        let cover_path = cover_path_for_disc(disc_info, album_dir);
        let total_tracks = disc_info.tracks.len() as u32;

        for (idx, file_path) in flac_files.iter().enumerate() {
            let Some(track) = disc_info.tracks.get(idx) else {
                break;
            };
            let track_num = idx as u32 + 1;

            if self.config.behavior.skip_existing
                && file_has_dizzylab_flac_tag(file_path, &disc_info.id)
            {
                debug!("FLAC已有完整标签，跳过: {}", file_path.display());
                continue;
            }

            // Save mtime before tag write so we can restore it afterwards.
            let saved_mtime = std::fs::metadata(file_path)
                .ok()
                .map(|m| filetime::FileTime::from_last_modification_time(&m));

            if let Err(e) = write_flac_tags(
                file_path,
                disc_info,
                track,
                track_num,
                total_tracks,
                &cover_path,
            ) {
                warn!("写入FLAC标签失败 {}: {}", file_path.display(), e);
            } else {
                debug!("已写入FLAC标签: {}", file_path.display());
                if let Some(ft) = saved_mtime {
                    if let Err(e) = set_file_times(file_path, ft, ft) {
                        warn!("还原FLAC时间戳失败 {}: {}", file_path.display(), e);
                    }
                }
            }
        }

        info!(
            "FLAC标签写入完成 ({} 个文件) - {}",
            flac_files.len().min(disc_info.tracks.len()),
            disc_info.title
        );
    }
}

fn file_has_dizzylab_flac_tag(file_path: &Path, disc_id: &str) -> bool {
    let Ok(tag) = metaflac::Tag::read_from_path(file_path) else {
        return false;
    };
    let Some(vc) = tag.vorbis_comments() else {
        return false;
    };
    vc.get("DIZZYLAB_ID")
        .and_then(|v| v.first())
        .map(|v| v == disc_id)
        .unwrap_or(false)
}

fn write_flac_tags(
    file_path: &Path,
    disc_info: &DiscInfo,
    track: &Track,
    track_num: u32,
    total_tracks: u32,
    cover_path: &Path,
) -> Result<()> {
    let mut tag = metaflac::Tag::read_from_path(file_path).unwrap_or_else(|_| metaflac::Tag::new());

    {
        let vc = tag.vorbis_comments_mut();
        vc.set_title(vec![track.title.clone()]);
        vc.set_album(vec![disc_info.title.clone()]);

        let artist = if track.authers.is_empty() {
            disc_info.label.clone()
        } else {
            track.authers.clone()
        };
        vc.set_artist(vec![artist]);
        vc.set("ALBUMARTIST", vec![disc_info.label.clone()]);

        vc.set_track(track_num);
        vc.set("TRACKTOTAL", vec![total_tracks.to_string()]);

        if let Some(date_str) = disc_info.release_date.as_deref() {
            let iso = normalize_date(date_str);
            vc.set("DATE", vec![iso]);
            if let Some(year) = extract_year_from_date(date_str) {
                vc.set("YEAR", vec![year.to_string()]);
            }
        }

        if !disc_info.tags.is_empty() {
            vc.set_genre(disc_info.tags.clone());
        }

        vc.set("DIZZYLAB_ID", vec![disc_info.id.clone()]);
        vc.set("BITRATE", vec!["FLAC".to_string()]);
    }

    if cover_path.exists() {
        if let Ok(data) = std::fs::read(cover_path) {
            let mime_type = if cover_path.extension().and_then(|e| e.to_str()) == Some("png") {
                "image/png".to_string()
            } else {
                "image/jpeg".to_string()
            };
            tag.remove_blocks(metaflac::BlockType::Picture);
            let mut pic = metaflac::block::Picture::new();
            pic.mime_type = mime_type;
            pic.picture_type = metaflac::block::PictureType::CoverFront;
            pic.data = data;
            tag.push_block(metaflac::Block::Picture(pic));
        }
    }

    tag.write_to_path(file_path)?;
    Ok(())
}

// ── MP3 / ID3 helpers ─────────────────────────────────────────────────────────

fn write_mp3_tags(
    file_path: &Path,
    disc_info: &DiscInfo,
    track: &Track,
    track_num: u32,
    total_tracks: u32,
    cover_path: &Path,
    format: &str,
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

    // Dizzylab disc ID and download format as custom TXXX frames.
    tag.add_frame(id3::frame::ExtendedText {
        description: "DIZZYLAB_ID".to_string(),
        value: disc_info.id.clone(),
    });
    tag.add_frame(id3::frame::ExtendedText {
        description: "BITRATE".to_string(),
        value: format.to_string(),
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
