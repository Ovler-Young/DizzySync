use super::Downloader;
use crate::archive::{self, ArchiveFormat};
use crate::types::DiscInfo;
use anyhow::{anyhow, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

impl Downloader {
    pub(super) async fn download_web_format(
        &self,
        disc_info: &DiscInfo,
        format: &str,
        album_dir: &Path,
    ) -> Result<()> {
        let target_dir = album_dir.to_path_buf();

        if self.config.behavior.skip_existing && target_dir.exists() {
            let ext = match format {
                "FLAC" => "flac",
                "128" | "320" => "mp3",
                _ => "",
            };
            if !ext.is_empty() {
                if let Ok(entries) = fs::read_dir(&target_dir) {
                    let has_audio = entries.filter_map(|e| e.ok()).any(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.to_lowercase().ends_with(&format!(".{ext}")))
                            .unwrap_or(false)
                    });
                    if has_audio {
                        info!("格式 {} 已存在，跳过下载 - {}", format, disc_info.title);
                        return Ok(());
                    }
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

    /// After gift extraction, find LRC files in the `gift/` subdirectory and copy them
    /// next to the corresponding audio files in `album_dir`.
    ///
    /// LRC filenames in the wild come in at least three forms:
    ///   1. `1.歌名.lrc`   — digit(s) + dot + title
    ///   2. `1 歌名.lrc`   — digit(s) + space + title
    ///   3. `歌名.lrc`     — bare title, no track number
    ///
    /// Matching priority: track number (if present) → normalized title comparison.
    pub(super) fn match_lrc_files(&self, disc_info: &DiscInfo, album_dir: &Path) {
        let gift_dir = album_dir.join("gift");
        if !gift_dir.exists() {
            return;
        }

        let lrc_files = collect_lrc_files(&gift_dir);
        if lrc_files.is_empty() {
            return;
        }

        info!("发现 {} 个LRC文件，尝试匹配曲目...", lrc_files.len());

        for (idx, track) in disc_info.tracks.iter().enumerate() {
            let track_num = idx + 1;
            let normalized_track = normalize_title(&track.title);

            let matched = lrc_files.iter().find(|lrc_path| {
                let stem = lrc_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let parsed = parse_lrc_stem(stem);

                if let Some(num) = parsed.track_num {
                    if num == track_num {
                        return true;
                    }
                }

                normalize_title(&parsed.title) == normalized_track
            });

            if let Some(lrc_path) = matched {
                let dest_name =
                    format!("{} {}.lrc", track_num, self.sanitize_filename(&track.title));
                let dest = album_dir.join(&dest_name);
                let src_display = lrc_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                match fs::copy(lrc_path, &dest) {
                    Ok(_) => info!("LRC匹配: {} → {}", src_display, dest_name),
                    Err(e) => warn!("复制LRC失败 {}: {}", dest_name, e),
                }
            }
        }
    }
}

struct ParsedLrc {
    track_num: Option<usize>,
    title: String,
}

/// Parse an LRC file stem (filename without extension) into an optional track
/// number and a title string.  Handles the three documented separator styles:
/// `{N}.{title}`, `{N} {title}`, and bare `{title}`.
fn parse_lrc_stem(stem: &str) -> ParsedLrc {
    let trimmed = stem.trim();
    let digit_end = trimmed
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0);

    if digit_end > 0 {
        if let Some(sep) = trimmed[digit_end..].chars().next() {
            if sep == '.' || sep == ' ' {
                if let Ok(num) = trimmed[..digit_end].parse::<usize>() {
                    // sep is ASCII so its encoded length is 1 byte
                    let title = trimmed[digit_end + 1..].trim().to_string();
                    if !title.is_empty() {
                        return ParsedLrc {
                            track_num: Some(num),
                            title,
                        };
                    }
                }
            }
        }
    }

    ParsedLrc {
        track_num: None,
        title: trimmed.to_string(),
    }
}

/// Case-insensitive title normalisation: collapse all non-alphanumeric characters
/// to single spaces so that punctuation / spacing differences don't break matching.
fn normalize_title(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Recursively collect every `.lrc` file under `dir`.
fn collect_lrc_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_lrc_files(&path));
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("lrc"))
            .unwrap_or(false)
        {
            result.push(path);
        }
    }
    result
}
