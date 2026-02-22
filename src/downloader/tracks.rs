use super::{file_md5_matches_etag, Downloader};
use crate::archive::filetime_from_http_date;
use crate::types::DiscInfo;
use anyhow::Result;
use filetime::set_file_times;
use std::fs::File;
use std::io::Write;
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

            // If the file exists, do a HEAD to compare MD5 via ETag.
            if self.config.behavior.skip_existing && file_path.exists() {
                match self.client.head_url(&cdn_url).await {
                    Ok(meta) => {
                        let md5_matches = meta
                            .etag
                            .as_deref()
                            .map(|etag| file_md5_matches_etag(&file_path, etag))
                            .unwrap_or(false);

                        if md5_matches {
                            // File is identical — only update mtime.
                            if let Some(date_str) = &meta.last_modified {
                                if let Some(ft) = filetime_from_http_date(date_str) {
                                    if let Err(e) = set_file_times(&file_path, ft, ft) {
                                        warn!("更新时间戳失败 {}: {}", file_name, e);
                                    } else {
                                        debug!("更新时间戳: {}", file_name);
                                    }
                                }
                            }
                            debug!("MD5一致，跳过下载: {}", file_name);
                            continue;
                        }

                        info!("MD5不一致，重新下载: {}", file_name);
                    }
                    Err(e) => {
                        warn!("HEAD 请求失败，重新下载 {}: {}", file_name, e);
                    }
                }
            }

            // Download the file.
            let (data, last_modified) = match self
                .client
                .download_bytes_with_last_modified(&cdn_url)
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    warn!("下载曲目 {} 失败: {}", track.title, e);
                    continue;
                }
            };

            let mut file = File::create(&file_path)?;
            file.write_all(&data)?;
            drop(file);

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
