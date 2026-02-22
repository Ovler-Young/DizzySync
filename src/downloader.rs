use crate::client::{DiscInfo, DiscListItem, DizzylabClient};
use crate::config::Config;
use anyhow::{anyhow, Result};
use chrono::{self, Datelike};
use encoding_rs::GBK;
use filetime::{set_file_times, FileTime};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use unrar::Archive;
use zip::ZipArchive;

#[derive(Debug, Clone, Copy)]
enum ArchiveFormat {
    Zip,
    Rar,
    Unknown,
}

#[derive(Clone)]
pub struct Downloader {
    client: DizzylabClient,
    config: Config,
    token: String,
}

impl Downloader {
    pub fn new(client: DizzylabClient, config: Config, token: String) -> Self {
        Self {
            client,
            config,
            token,
        }
    }

    pub async fn sync_all_albums(&self, albums: Vec<DiscListItem>) -> Result<()> {
        let total_albums = albums.len();
        info!("开始同步 {} 个专辑", total_albums);

        fs::create_dir_all(&self.config.paths.output_dir)?;

        let concurrency = if self.config.behavior.single_threaded {
            1
        } else {
            self.config.behavior.max_concurrent_albums.max(1)
        };

        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
        let this = Arc::new(self.clone());
        let mut join_set = tokio::task::JoinSet::new();

        for (index, disc_item) in albums.into_iter().enumerate() {
            let sem = semaphore.clone();
            let downloader = this.clone();
            join_set.spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                info!(
                    "处理专辑 {}/{}: {} - {}",
                    index + 1,
                    total_albums,
                    disc_item.title,
                    disc_item.label
                );

                let disc_info = match downloader
                    .client
                    .get_disc_info(&disc_item.id, &downloader.token)
                    .await
                {
                    Ok(info) => info,
                    Err(e) => {
                        error!("获取专辑 {} 详情失败: {}", disc_item.id, e);
                        return;
                    }
                };

                if let Err(e) = downloader.download_album(&disc_info).await {
                    error!("下载专辑 {} 失败: {}", disc_info.id, e);
                }

                if downloader.config.behavior.single_threaded {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            });
        }

        while let Some(res) = join_set.join_next().await {
            if let Err(e) = res {
                error!("任务异常: {}", e);
            }
        }

        info!("同步完成！");
        Ok(())
    }

    /// Download a single album given its full disc info (already fetched).
    pub async fn download_album(&self, disc_info: &DiscInfo) -> Result<()> {
        let album_dir = self.get_album_directory(disc_info);
        info!("album_dir: {}", album_dir.display());

        fs::create_dir_all(&album_dir)?;

        if self.config.behavior.generate_readme {
            if let Err(e) = self.generate_readme(disc_info, &album_dir) {
                warn!("生成README失败: {}", e);
            }
        }

        if self.config.behavior.generate_nfo {
            if let Err(e) = self.generate_nfo(disc_info, &album_dir) {
                warn!("生成NFO失败: {}", e);
            }
        }

        if let Err(e) = self.download_cover(disc_info, &album_dir).await {
            warn!("下载封面失败: {}", e);
        }

        if self.config.behavior.metadata_only {
            info!("仅下载元数据模式：跳过音频文件下载 - {}", disc_info.title);
            return Ok(());
        }

        for format in &self.config.download.formats {
            if let Err(e) = self.download_format(disc_info, format, &album_dir).await {
                warn!("下载格式 {} 失败: {}", format, e);
            }
        }

        Ok(())
    }

    async fn download_format(
        &self,
        disc_info: &DiscInfo,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        if format == "gift" {
            return self.download_gift(disc_info, album_dir).await;
        }

        if format == "FLAC" {
            return self.download_web_format(disc_info, format, album_dir).await;
        }

        self.download_tracks_for_format(disc_info, format, album_dir)
            .await
    }

    async fn download_web_format(
        &self,
        disc_info: &DiscInfo,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        let target_dir = if self.config.download.flatten {
            album_dir.to_path_buf()
        } else {
            album_dir.join(format)
        };

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

        let archive_format = self.detect_archive_format(&file_data);
        match archive_format {
            ArchiveFormat::Zip => {
                self.extract_zip_file(&file_data, format, album_dir)?;
            }
            ArchiveFormat::Rar => {
                self.extract_rar_file(&file_data, &disc_info.id, format, album_dir)?;
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

    async fn download_tracks_for_format(
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

        let target_dir = if self.config.download.flatten {
            album_dir.to_path_buf()
        } else {
            album_dir.join(format)
        };

        // Skip if directory already has audio files
        if self.config.behavior.skip_existing && target_dir.exists() {
            if let Ok(entries) = fs::read_dir(&target_dir) {
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

        fs::create_dir_all(&target_dir)?;

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

            // Small delay between tracks to avoid hammering the server
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn download_gift(&self, disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
        if !disc_info.hasgift {
            info!("专辑 {} 没有特典内容，跳过", disc_info.title);
            return Ok(());
        }

        // Check skip_existing for gift
        let target_dir = if self.config.download.flatten {
            album_dir.to_path_buf()
        } else {
            album_dir.join("gift")
        };

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

        let archive_format = self.detect_archive_format(&file_data);
        match archive_format {
            ArchiveFormat::Zip => {
                self.extract_zip_file(&file_data, "gift", album_dir)?;
            }
            ArchiveFormat::Rar => {
                self.extract_rar_file(&file_data, &disc_info.id, "gift", album_dir)?;
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

    fn get_album_directory(&self, disc_info: &DiscInfo) -> PathBuf {
        let directory_name = self.generate_directory_name(disc_info);
        self.config.paths.output_dir.join(directory_name)
    }

    fn generate_directory_name(&self, disc_info: &DiscInfo) -> String {
        let template = &self.config.paths.directory_template;

        let current_year = chrono::Utc::now().year().to_string();
        let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let mut result = template.clone();
        result = result.replace("{album}", &self.sanitize_filename(&disc_info.title));
        result = result.replace("{label}", &self.sanitize_filename(&disc_info.label));

        let authors = disc_info
            .tracks
            .first()
            .map(|t| t.authers.as_str())
            .unwrap_or(&disc_info.label);
        result = result.replace("{authors}", &self.sanitize_filename(authors));

        let year = disc_info
            .release_date
            .as_deref()
            .and_then(extract_year_from_date)
            .unwrap_or_else(|| current_year.clone());
        result = result.replace("{year}", &year);

        let date = disc_info
            .release_date
            .as_ref()
            .map(|d| self.normalize_date(d))
            .unwrap_or(current_date);
        result = result.replace("{date}", &date);

        result
    }

    fn normalize_date(&self, date_str: &str) -> String {
        // Convert "2025年6月10日" → "2025-06-10", or pass through ISO dates
        if let Some(caps) = regex::Regex::new(r"(\d{4})年(\d{1,2})月(\d{1,2})日")
            .unwrap()
            .captures(date_str)
        {
            if let (Some(y), Some(m), Some(d)) = (caps.get(1), caps.get(2), caps.get(3)) {
                return format!(
                    "{}-{:02}-{:02}",
                    y.as_str(),
                    m.as_str().parse::<u32>().unwrap_or(1),
                    d.as_str().parse::<u32>().unwrap_or(1)
                );
            }
        }
        date_str.to_string()
    }

    fn sanitize_filename(&self, name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn get_cover_extension(&self, cover_url: &str) -> &str {
        if cover_url.contains(".jpg") || cover_url.contains(".jpeg") {
            "jpg"
        } else if cover_url.contains(".png") {
            "png"
        } else if cover_url.contains(".webp") {
            "webp"
        } else if cover_url.contains(".gif") {
            "gif"
        } else {
            "jpg"
        }
    }

    fn generate_readme(&self, disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
        let template = self
            .load_readme_template()
            .unwrap_or_else(|_| self.get_default_readme_template());
        let content = self.apply_template_variables(&template, disc_info);
        fs::write(album_dir.join("README.md"), content)?;
        Ok(())
    }

    fn generate_nfo(&self, disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
        let content = self.generate_nfo_content(disc_info);
        fs::write(album_dir.join("album.nfo"), content)?;
        Ok(())
    }

    async fn download_cover(&self, disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
        if disc_info.cover.is_empty() {
            debug!("专辑 {} 没有封面URL，跳过下载", disc_info.title);
            return Ok(());
        }

        info!("下载封面: {}", disc_info.title);
        let cover_data = self
            .client
            .download_cover(&disc_info.cover, &disc_info.id)
            .await?;

        let extension = self.get_cover_extension(&disc_info.cover);
        fs::write(album_dir.join(format!("cover.{extension}")), cover_data)?;
        info!("封面下载完成: {}", disc_info.title);

        Ok(())
    }

    fn load_readme_template(&self) -> Result<String> {
        Ok(fs::read_to_string("readme.template.md")?)
    }

    fn get_default_readme_template(&self) -> String {
        r#"# {album}

**厂牌:** {label}
**发布日期:** {release_date}
**专辑ID:** {id}

## 描述

{description}

## 标签

{tags}

## 下载信息

- **下载时间:** {download_date}
- **下载格式:** {formats}

---

*由 DizzySync 自动生成*
"#
        .to_string()
    }

    fn apply_template_variables(&self, template: &str, disc_info: &DiscInfo) -> String {
        let mut result = template.to_string();

        result = result.replace("{album}", &disc_info.title);
        result = result.replace("{label}", &disc_info.label);
        result = result.replace("{id}", &disc_info.id);
        result = result.replace("{cover}", &disc_info.cover);
        result = result.replace(
            "{release_date}",
            disc_info.release_date.as_deref().unwrap_or("未知"),
        );
        let description = disc_info.disc_description.as_deref().unwrap_or("暂无描述");
        result = result.replace("{description}", description);
        result = result.replace("{tags}", &disc_info.tags.join(", "));
        let authors = disc_info
            .tracks
            .first()
            .map(|t| t.authers.as_str())
            .unwrap_or(&disc_info.label);
        result = result.replace("{authors}", authors);
        let year = disc_info
            .release_date
            .as_deref()
            .and_then(extract_year_from_date)
            .unwrap_or_else(|| "未知".to_string());
        result = result.replace("{year}", &year);
        result = result.replace(
            "{download_date}",
            &chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );
        result = result.replace("{formats}", &self.config.download.formats.join(", "));

        result
    }

    fn generate_nfo_content(&self, disc_info: &DiscInfo) -> String {
        let authors = disc_info
            .tracks
            .first()
            .map(|t| t.authers.as_str())
            .unwrap_or(&disc_info.label);
        let year = disc_info
            .release_date
            .as_deref()
            .and_then(extract_year_from_date)
            .unwrap_or_else(|| "Unknown".to_string());

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<album>
    <title>{}</title>
    <artist>{}</artist>
    <genre>{}</genre>
    <year>{}</year>
    <releasedate>{}</releasedate>
    <label>{}</label>
    <id>{}</id>
    <plot>{}</plot>
    <tags>
        {}
    </tags>
    <source>Dizzylab</source>
    <url>https://www.dizzylab.net/d/{}/</url>
</album>"#,
            disc_info.title,
            authors,
            disc_info
                .tags
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Music"),
            year,
            disc_info.release_date.as_deref().unwrap_or("Unknown"),
            disc_info.label,
            disc_info.id,
            disc_info.disc_description.as_deref().unwrap_or(""),
            disc_info
                .tags
                .iter()
                .map(|tag| format!("        <tag>{tag}</tag>"))
                .collect::<Vec<_>>()
                .join("\n"),
            disc_info.id
        )
    }

    fn extract_zip_file(&self, zip_data: &[u8], format: &str, album_dir: &Path) -> Result<()> {
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            let file_name_raw = file.name_raw();
            let file_name: Cow<str> = match std::str::from_utf8(file_name_raw) {
                Ok(name) => Cow::Borrowed(name),
                Err(_) => GBK.decode(file_name_raw).0,
            };

            if file_name.ends_with('/') {
                continue;
            }

            debug!("解压文件: {}", file_name);

            let output_path = if self.config.download.flatten {
                album_dir.join(&*file_name)
            } else {
                let format_dir = album_dir.join(format);
                fs::create_dir_all(&format_dir)?;
                format_dir.join(&*file_name)
            };

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let zip_last_modified = file.last_modified();
            let mut output_file = File::create(&output_path)?;
            std::io::copy(&mut file, &mut output_file)?;
            drop(output_file);

            if let Err(e) = self.set_file_timestamps(&output_path, zip_last_modified) {
                warn!("设置文件时间戳失败 {}: {}", output_path.display(), e);
            }
        }

        Ok(())
    }

    fn detect_archive_format(&self, data: &[u8]) -> ArchiveFormat {
        if data.len() < 4 {
            return ArchiveFormat::Unknown;
        }

        if data.starts_with(b"PK") {
            return ArchiveFormat::Zip;
        }

        if data.len() >= 8 && &data[0..8] == b"Rar!\x1a\x07\x01\x00" {
            return ArchiveFormat::Rar;
        }
        if data.len() >= 7 && &data[0..7] == b"Rar!\x1a\x07\x00" {
            return ArchiveFormat::Rar;
        }

        ArchiveFormat::Unknown
    }

    fn extract_rar_file(
        &self,
        rar_data: &[u8],
        album_id: &str,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        let temp_file_path = album_dir.join(format!("temp_{album_id}.rar"));
        fs::write(&temp_file_path, rar_data)?;

        let archive = Archive::new(&temp_file_path);
        let archive = archive.open_for_processing()?;

        self.process_rar_archive(archive, format, album_dir)?;

        if let Err(e) = fs::remove_file(&temp_file_path) {
            warn!("删除临时RAR文件失败: {}", e);
        }

        Ok(())
    }

    fn process_rar_archive(
        &self,
        mut archive: unrar::OpenArchive<unrar::Process, unrar::CursorBeforeHeader>,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        loop {
            match archive.read_header() {
                Ok(Some(header_archive)) => {
                    let entry = header_archive.entry();
                    let filename = &entry.filename;

                    if entry.is_directory() {
                        archive = header_archive.skip()?;
                        continue;
                    }

                    debug!("解压RAR文件: {}", filename.display());

                    let output_path = if self.config.download.flatten {
                        album_dir.join(filename)
                    } else {
                        let format_dir = album_dir.join(format);
                        fs::create_dir_all(&format_dir)?;
                        format_dir.join(filename)
                    };

                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    let (data, next_archive) = header_archive.read()?;
                    fs::write(&output_path, data)?;

                    archive = next_archive;
                }
                Ok(None) => break,
                Err(e) => {
                    error!("读取RAR头部失败: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn set_file_timestamps(&self, file_path: &Path, zip_datetime: zip::DateTime) -> Result<()> {
        let year = zip_datetime.year() as i32;
        let month = zip_datetime.month() as u32;
        let day = zip_datetime.day() as u32;
        let hour = zip_datetime.hour() as u32;
        let minute = zip_datetime.minute() as u32;
        let second = zip_datetime.second() as u32;

        if let Some(naive_date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
            if let Some(naive_datetime) = naive_date.and_hms_opt(hour, minute, second) {
                let unix_timestamp = naive_datetime.and_utc().timestamp();
                let filetime = FileTime::from_unix_time(unix_timestamp, 0);

                if let Err(e) = set_file_times(file_path, filetime, filetime) {
                    return Err(anyhow!("设置文件时间戳失败: {}", e));
                }

                debug!(
                    "已设置ZIP文件时间戳: {} -> {}",
                    file_path.display(),
                    naive_datetime
                );
            }
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

fn extract_year_from_date(date_str: &str) -> Option<String> {
    // "2025年6月10日" or "2025-06-10" or "2025/06/10"
    if let Some(caps) = regex::Regex::new(r"(\d{4})年").unwrap().captures(date_str) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    if let Some(caps) = regex::Regex::new(r"^(\d{4})[/-]")
        .unwrap()
        .captures(date_str)
    {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    None
}
