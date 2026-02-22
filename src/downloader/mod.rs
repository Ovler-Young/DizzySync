mod tracks;
mod web;

use crate::archive;
use crate::client::DizzylabClient;
use crate::config::Config;
use crate::metadata;
use crate::types::{DiscInfo, DiscListItem};
use anyhow::Result;
use chrono::{self, Datelike};
use filetime::set_file_times;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct Downloader {
    pub(super) client: DizzylabClient,
    pub(super) config: Config,
    pub(super) token: String,
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
            if let Err(e) =
                metadata::generate_readme(disc_info, &album_dir, &self.config.download.formats)
            {
                warn!("生成README失败: {}", e);
            }
        }

        if self.config.behavior.generate_nfo {
            if let Err(e) = metadata::generate_nfo(disc_info, &album_dir) {
                warn!("生成NFO失败: {}", e);
            }
        }

        let cover_last_modified = match self.download_cover(disc_info, &album_dir).await {
            Ok(lm) => lm,
            Err(e) => {
                warn!("下载封面失败: {}", e);
                None
            }
        };

        if self.config.behavior.metadata_only {
            info!("仅下载元数据模式：跳过音频文件下载 - {}", disc_info.title);
            self.set_metadata_timestamps(disc_info, &album_dir, cover_last_modified.as_deref());
            return Ok(());
        }

        for format in &self.config.download.formats {
            if let Err(e) = self.download_format(disc_info, format, &album_dir).await {
                warn!("下载格式 {} 失败: {}", format, e);
            }
        }

        self.match_lrc_files(disc_info, &album_dir);

        // Set timestamps on metadata files and the album directory last,
        // after all file operations, so directory mtime is not overwritten.
        self.set_metadata_timestamps(disc_info, &album_dir, cover_last_modified.as_deref());

        Ok(())
    }

    /// Set modification timestamps of cover, README, NFO, and the album directory.
    /// Priority: server `Last-Modified` > album `release_date`.
    /// Must be called after all other file writes so the directory mtime is set last.
    fn set_metadata_timestamps(
        &self,
        disc_info: &DiscInfo,
        album_dir: &Path,
        server_last_modified: Option<&str>,
    ) {
        // Priority 1: Last-Modified from OSS/CDN
        let ft = server_last_modified.and_then(archive::filetime_from_http_date);
        // Priority 2: release_date from disc info
        let ft = ft.or_else(|| {
            disc_info
                .release_date
                .as_deref()
                .and_then(archive::filetime_from_release_date)
        });
        let Some(ft) = ft else {
            debug!("专辑 {} 无可用时间戳，跳过时间戳设置", disc_info.title);
            return;
        };

        let cover_ext = get_cover_extension(&disc_info.cover);
        let metadata_files: &[(&str, bool)] = &[
            (&format!("cover.{cover_ext}"), true),
            ("README.md", self.config.behavior.generate_readme),
            ("album.nfo", self.config.behavior.generate_nfo),
        ];

        for (name, enabled) in metadata_files {
            if !enabled {
                continue;
            }
            let path = album_dir.join(name);
            if path.exists() {
                if let Err(e) = set_file_times(&path, ft, ft) {
                    warn!("设置 {} 时间戳失败: {}", name, e);
                } else {
                    debug!("已设置 {} 时间戳", name);
                }
            }
        }

        // Directory timestamp must come last (any file write inside updates it).
        if let Err(e) = set_file_times(album_dir, ft, ft) {
            warn!("设置专辑目录时间戳失败: {}", e);
        } else {
            debug!("已设置专辑目录时间戳");
        }
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

    /// Download (or skip) the cover image.
    /// Returns the server `Last-Modified` value when available, whether or not the file was re-downloaded.
    async fn download_cover(
        &self,
        disc_info: &DiscInfo,
        album_dir: &Path,
    ) -> Result<Option<String>> {
        if disc_info.cover.is_empty() {
            debug!("专辑 {} 没有封面URL，跳过下载", disc_info.title);
            return Ok(None);
        }

        let extension = get_cover_extension(&disc_info.cover);
        let cover_path = album_dir.join(format!("cover.{extension}"));

        // HEAD first: cheap check for ETag (MD5) and Last-Modified.
        let meta = match self
            .client
            .head_cover(&disc_info.cover, &disc_info.id)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                warn!("封面 HEAD 请求失败，直接下载: {}", e);
                // Fall through to unconditional download with no metadata.
                let (data, m) = self
                    .client
                    .download_cover(&disc_info.cover, &disc_info.id)
                    .await?;
                fs::write(&cover_path, data)?;
                info!("封面下载完成: {}", disc_info.title);
                return Ok(m.last_modified);
            }
        };

        // If local file exists and ETag matches, skip download.
        if cover_path.exists() {
            if let Some(etag) = &meta.etag {
                if cover_md5_matches_etag(&cover_path, etag) {
                    info!("封面未变更（MD5匹配），跳过下载: {}", disc_info.title);
                    return Ok(meta.last_modified);
                }
            }
        }

        info!("下载封面: {}", disc_info.title);
        let (data, _) = self
            .client
            .download_cover(&disc_info.cover, &disc_info.id)
            .await?;
        fs::write(&cover_path, data)?;
        info!("封面下载完成: {}", disc_info.title);

        Ok(meta.last_modified)
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
            .and_then(metadata::extract_year_from_date)
            .unwrap_or_else(|| current_year.clone());
        result = result.replace("{year}", &year);

        let date = disc_info
            .release_date
            .as_ref()
            .map(|d| normalize_date(d))
            .unwrap_or(current_date);
        result = result.replace("{date}", &date);

        result
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
}

/// Returns true if the local file's MD5 matches the ETag from the server.
/// OSS ETags for single-part uploads are hex MD5 (with surrounding quotes).
/// Multipart-upload ETags contain a hyphen ("MD5-N") and are skipped.
fn cover_md5_matches_etag(file_path: &Path, etag: &str) -> bool {
    let etag_clean = etag.trim_matches('"');
    if etag_clean.contains('-') {
        // Multipart ETag — not a plain MD5, cannot compare.
        return false;
    }
    match fs::read(file_path) {
        Ok(data) => {
            let digest = md5::compute(&data);
            format!("{digest:X}").eq_ignore_ascii_case(etag_clean)
        }
        Err(_) => false,
    }
}

fn normalize_date(date_str: &str) -> String {
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

fn get_cover_extension(cover_url: &str) -> &str {
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
