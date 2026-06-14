use anyhow::{anyhow, Result};
use encoding_rs::GBK;
use filetime::{set_file_times, FileTime};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use tracing::{debug, warn};
use unrar::Archive;
use zip::ZipArchive;

#[derive(Debug, Clone, Copy)]
pub enum ArchiveFormat {
    Zip,
    Rar,
    Unknown,
}

pub fn detect_archive_format(data: &[u8]) -> ArchiveFormat {
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

/// Detect archive format by reading the first few bytes of a file on disk.
pub fn detect_archive_format_from_path(path: &Path) -> ArchiveFormat {
    let Ok(mut file) = File::open(path) else {
        return ArchiveFormat::Unknown;
    };
    let mut buf = [0u8; 8];
    let n = file.read(&mut buf).unwrap_or(0);
    detect_archive_format(&buf[..n])
}

/// Extract a ZIP archive from a file path on disk.
pub fn extract_zip_from_path(zip_path: &Path, format: &str, album_dir: &Path) -> Result<()> {
    let file = File::open(zip_path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let file_name_raw = entry.name_raw().to_vec();
        let file_name: Cow<str> = match std::str::from_utf8(&file_name_raw) {
            Ok(name) => Cow::Owned(name.to_string()),
            Err(_) => Cow::Owned(GBK.decode(&file_name_raw).0.into_owned()),
        };

        if file_name.ends_with('/') {
            continue;
        }

        let Some(safe_name) = sanitize_archive_path(file_name.as_ref()) else {
            warn!("跳过不安全的ZIP路径: {}", file_name);
            continue;
        };

        debug!("解压文件: {}", safe_name.display());

        let output_path = if format == "gift" {
            let format_dir = album_dir.join(format);
            fs::create_dir_all(&format_dir)?;
            format_dir.join(&safe_name)
        } else {
            album_dir.join(&safe_name)
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let zip_last_modified = entry.last_modified();
        let mut output_file = File::create(&output_path)?;
        std::io::copy(&mut entry, &mut output_file)?;
        drop(output_file);

        if let Some(dt) = zip_last_modified {
            if let Err(e) = set_file_timestamps(&output_path, dt) {
                warn!("设置文件时间戳失败 {}: {}", output_path.display(), e);
            }
        }
    }

    Ok(())
}

/// Extract a RAR archive from a file path on disk.
pub fn extract_rar_from_path(rar_path: &Path, format: &str, album_dir: &Path) -> Result<()> {
    let archive = Archive::new(rar_path);
    let archive = archive.open_for_processing()?;
    process_rar_archive(archive, format, album_dir)
}

fn process_rar_archive(
    archive: unrar::OpenArchive<unrar::Process, unrar::CursorBeforeHeader>,
    format: &str,
    album_dir: &Path,
) -> Result<()> {
    let mut next_archive = Some(archive);

    while let Some(archive) = next_archive.take() {
        match archive.read_header()? {
            Some(header_archive) => {
                let entry = header_archive.entry();
                let filename = &entry.filename;

                if entry.is_directory() {
                    next_archive = Some(header_archive.skip()?);
                    continue;
                }

                let Some(safe_name) = sanitize_archive_path(filename) else {
                    warn!("跳过不安全的RAR路径: {}", filename.display());
                    next_archive = Some(header_archive.skip()?);
                    continue;
                };

                debug!("解压RAR文件: {}", safe_name.display());

                let output_path = if format == "gift" {
                    let format_dir = album_dir.join(format);
                    fs::create_dir_all(&format_dir)?;
                    format_dir.join(&safe_name)
                } else {
                    album_dir.join(&safe_name)
                };

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let (data, archive) = header_archive.read()?;
                fs::write(&output_path, data)?;
                next_archive = Some(archive);
            }
            None => break,
        }
    }

    Ok(())
}

fn sanitize_archive_path(path: impl AsRef<Path>) -> Option<PathBuf> {
    let mut safe_path = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::Normal(part) => safe_path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if safe_path.as_os_str().is_empty() {
        None
    } else {
        Some(safe_path)
    }
}

/// Parse an HTTP `Last-Modified` header value (RFC 2822) into a FileTime.
/// Example input: `"Fri, 20 Feb 2026 02:15:26 GMT"`
pub fn filetime_from_http_date(date_str: &str) -> Option<FileTime> {
    let dt = chrono::DateTime::parse_from_rfc2822(date_str).ok()?;
    Some(FileTime::from_unix_time(dt.timestamp(), 0))
}

/// Parse an album release_date string into a FileTime suitable for `set_file_times`.
/// Delegates format normalization to `metadata::normalize_date`.
pub fn filetime_from_release_date(date_str: &str) -> Option<FileTime> {
    use chrono::NaiveDate;
    let iso = crate::metadata::normalize_date(date_str);
    let mut parts = iso.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    let dt = NaiveDate::from_ymd_opt(y, m, d)?.and_hms_opt(0, 0, 0)?;
    Some(FileTime::from_unix_time(dt.and_utc().timestamp(), 0))
}

pub fn set_file_timestamps(file_path: &Path, zip_datetime: zip::DateTime) -> Result<()> {
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
                return Err(anyhow!("设置文件时间戳失败: {e}"));
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
