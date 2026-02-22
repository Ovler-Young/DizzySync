use anyhow::{anyhow, Result};
use encoding_rs::GBK;
use filetime::{set_file_times, FileTime};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::Cursor;
use std::path::Path;
use tracing::{debug, error, warn};
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

pub fn extract_zip_file(zip_data: &[u8], format: &str, album_dir: &Path) -> Result<()> {
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

        let output_path = if format == "gift" {
            let format_dir = album_dir.join(format);
            fs::create_dir_all(&format_dir)?;
            format_dir.join(&*file_name)
        } else {
            album_dir.join(&*file_name)
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let zip_last_modified = file.last_modified();
        let mut output_file = File::create(&output_path)?;
        std::io::copy(&mut file, &mut output_file)?;
        drop(output_file);

        if let Err(e) = set_file_timestamps(&output_path, zip_last_modified) {
            warn!("设置文件时间戳失败 {}: {}", output_path.display(), e);
        }
    }

    Ok(())
}

pub fn extract_rar_file(
    rar_data: &[u8],
    album_id: &str,
    format: &str,
    album_dir: &Path,
) -> Result<()> {
    let temp_file_path = album_dir.join(format!("temp_{album_id}.rar"));
    fs::write(&temp_file_path, rar_data)?;

    let archive = Archive::new(&temp_file_path);
    let archive = archive.open_for_processing()?;

    process_rar_archive(archive, format, album_dir)?;

    if let Err(e) = fs::remove_file(&temp_file_path) {
        warn!("删除临时RAR文件失败: {}", e);
    }

    Ok(())
}

fn process_rar_archive(
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

                let output_path = if format == "gift" {
                    let format_dir = album_dir.join(format);
                    fs::create_dir_all(&format_dir)?;
                    format_dir.join(filename)
                } else {
                    album_dir.join(filename)
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
