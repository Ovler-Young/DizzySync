use super::Downloader;
use crate::types::DiscInfo;
use anyhow::Result;
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

        if self.config.behavior.skip_existing && target_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&target_dir) {
                let has_audio = entries.filter_map(|e| e.ok()).any(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| {
                            let n = n.to_lowercase();
                            n.ends_with(".mp3") || n.ends_with(".flac")
                        })
                        .unwrap_or(false)
                });
                if has_audio {
                    info!("格式 {} 已存在，跳过下载 - {}", format, disc_info.title);
                    return Ok(());
                }
            }
        }

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
                "{:02} - {}.{}",
                track_num,
                self.sanitize_filename(&track.title),
                ext
            );
            let file_path = target_dir.join(&file_name);

            if self.config.behavior.skip_existing && file_path.exists() {
                debug!("曲目已存在，跳过: {}", file_name);
                continue;
            }

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

            let data = match self.client.download_bytes(&cdn_url).await {
                Ok(d) => d,
                Err(e) => {
                    warn!("下载曲目 {} 失败: {}", track.title, e);
                    continue;
                }
            };

            let mut file = File::create(&file_path)?;
            file.write_all(&data)?;
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
