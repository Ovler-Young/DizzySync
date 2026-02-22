use super::{file_md5_matches_etag, Downloader};
use crate::archive::filetime_from_http_date;
use crate::types::DiscInfo;
use anyhow::Result;
use filetime::set_file_times;
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
                        let etag = meta.etag.as_deref();
                        // A plain-MD5 ETag has no hyphen; multipart ETags look like "abc123-5".
                        let etag_is_comparable = etag
                            .map(|e| !e.trim_matches('"').contains('-'))
                            .unwrap_or(false);

                        let should_skip = if etag_is_comparable {
                            file_md5_matches_etag(&file_path, etag.unwrap())
                        } else {
                            // Multipart ETag or no ETag: can't verify MD5.
                            // Skip if the file already exists with content.
                            file_path.metadata().map(|m| m.len() > 0).unwrap_or(false)
                        };

                        if should_skip {
                            if etag_is_comparable {
                                debug!("MD5一致，跳过下载: {}", file_name);
                            } else {
                                debug!("无法验证MD5（分段ETag），文件非空，跳过: {}", file_name);
                            }
                            // Update mtime regardless.
                            if let Some(date_str) = &meta.last_modified {
                                if let Some(ft) = filetime_from_http_date(date_str) {
                                    if let Err(e) = set_file_times(&file_path, ft, ft) {
                                        warn!("更新时间戳失败 {}: {}", file_name, e);
                                    } else {
                                        debug!("更新时间戳: {}", file_name);
                                    }
                                }
                            }
                            continue;
                        }

                        if etag_is_comparable {
                            info!("MD5不一致，重新下载: {}", file_name);
                        }
                        // else: file is empty — fall through to re-download silently.
                    }
                    Err(e) => {
                        warn!("HEAD 请求失败，重新下载 {}: {}", file_name, e);
                    }
                }
            }

            // Stream directly to the destination file (no in-memory buffer).
            let last_modified = match self.client.stream_to_file(&cdn_url, &file_path).await {
                Ok(lm) => lm,
                Err(e) => {
                    warn!("下载曲目 {} 失败: {}", track.title, e);
                    continue;
                }
            };

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
