use super::Downloader;
use crate::archive::{self, ArchiveFormat};
use crate::types::DiscInfo;
use anyhow::{anyhow, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::info;

impl Downloader {
    pub(super) async fn download_web_format(
        &self,
        disc_info: &DiscInfo,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        let target_dir = album_dir.to_path_buf();

        if self.config.behavior.skip_existing && target_dir.exists() {
            if let Ok(entries) = fs::read_dir(&target_dir) {
                if entries.count() > 0 {
                    info!("格式 {} 已存在，跳过下载 - {}", format, disc_info.title);
                    return Ok(());
                }
            }
        }

        let download_url = self
            .client
            .get_web_format_download_link(&disc_info.id, format)
            .await?;

        info!("下载格式 {} (web) - {}", format, disc_info.title);
        let file_data = self
            .client
            .download_file(&download_url, &disc_info.id)
            .await?;

        match archive::detect_archive_format(&file_data) {
            ArchiveFormat::Zip => archive::extract_zip_file(&file_data, format, album_dir)?,
            ArchiveFormat::Rar => {
                archive::extract_rar_file(&file_data, &disc_info.id, format, album_dir)?
            }
            ArchiveFormat::Unknown => {
                fs::create_dir_all(&target_dir)?;
                let file_path = target_dir.join(format!("{}_{}.bin", disc_info.id, format));
                let mut file = File::create(file_path)?;
                file.write_all(&file_data)?;
            }
        }

        Ok(())
    }

    pub(super) async fn download_gift(&self, disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
        if !disc_info.hasgift {
            info!("专辑 {} 没有特典内容，跳过", disc_info.title);
            return Ok(());
        }

        let target_dir = album_dir.join("gift");

        if self.config.behavior.skip_existing && target_dir.exists() {
            if let Ok(entries) = fs::read_dir(&target_dir) {
                if entries.count() > 0 {
                    info!("gift 已存在，跳过下载 - {}", disc_info.title);
                    return Ok(());
                }
            }
        }

        let links = self.client.get_gift_download_link(&disc_info.id).await?;
        if links.is_empty() {
            return Ok(());
        }

        let download_url = links
            .get("gift")
            .ok_or_else(|| anyhow!("无法获取 gift 下载链接"))?;

        let file_data = self
            .client
            .download_file(download_url, &disc_info.id)
            .await?;

        match archive::detect_archive_format(&file_data) {
            ArchiveFormat::Zip => archive::extract_zip_file(&file_data, "gift", album_dir)?,
            ArchiveFormat::Rar => {
                archive::extract_rar_file(&file_data, &disc_info.id, "gift", album_dir)?
            }
            ArchiveFormat::Unknown => {
                fs::create_dir_all(&target_dir)?;
                let file_path = target_dir.join(format!("gift_{}", disc_info.id));
                let mut file = File::create(file_path)?;
                file.write_all(&file_data)?;
            }
        }

        Ok(())
    }
}
