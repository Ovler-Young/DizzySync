use crate::config::Config;
use crate::metadata;
use crate::types::{DiscInfo, DiscListItem, LocalAlbumState, LocalTrackState};
use chrono::{self, Datelike};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn annotate_album_list(config: &Config, albums: &mut [DiscListItem]) {
    let index = build_album_index(&config.paths.output_dir);
    for album in albums {
        let expected_dir = album_directory_for_list_item(config, album);
        let indexed_dir = index.get(&album.id).cloned();
        let album_dir = if expected_dir.exists() {
            expected_dir
        } else if let Some(path) = indexed_dir {
            path
        } else {
            expected_dir
        };
        album.local = Some(album_state_from_dir(config, &album_dir, None));
    }
}

pub fn annotate_disc_info(config: &Config, album: &mut DiscInfo) {
    let album_dir = album_directory_for_disc(config, album);
    let state = album_state_from_dir(config, &album_dir, Some(album));
    album.local = Some(state);

    for (idx, track) in album.tracks.iter_mut().enumerate() {
        track.local = Some(track_state_from_dir(
            config,
            &album_dir,
            track.title.as_str(),
            idx + 1,
        ));
    }
}

fn album_state_from_dir(
    config: &Config,
    album_dir: &Path,
    album: Option<&DiscInfo>,
) -> LocalAlbumState {
    let directory_exists = album_dir.is_dir();
    let mut audio_files = 0usize;
    let mut gift_exists = false;
    let mut formats = BTreeMap::new();

    if directory_exists {
        for format in &config.download.formats {
            let present = match format.as_str() {
                "gift" => album_dir.join("gift").is_dir(),
                "FLAC" => count_extension(album_dir, "flac") > 0,
                "128" | "320" => count_extension(album_dir, "mp3") > 0,
                other => count_extension(album_dir, extension_for_format(other)) > 0,
            };
            formats.insert(format.clone(), present);
        }
        audio_files = count_audio_files(album_dir);
        gift_exists = album_dir.join("gift").is_dir();
    }

    let expected_tracks = album.map(|disc| disc.tracks.len()).unwrap_or(0);
    let downloaded_tracks = album
        .map(|disc| {
            disc.tracks
                .iter()
                .enumerate()
                .filter(|(idx, track)| {
                    track_state_from_dir(config, album_dir, track.title.as_str(), *idx + 1)
                        .downloaded
                })
                .count()
        })
        .unwrap_or(0);

    let has_audio = audio_files > 0;
    let has_metadata = directory_exists
        && (album_dir.join("README.md").exists()
            || album_dir.join("album.nfo").exists()
            || has_cover(album_dir));
    let downloaded = if expected_tracks > 0 {
        downloaded_tracks >= expected_tracks
    } else {
        has_audio || has_metadata || gift_exists
    };

    LocalAlbumState {
        downloaded,
        directory_exists,
        path: album_dir.display().to_string(),
        audio_files,
        expected_tracks,
        downloaded_tracks,
        gift_exists,
        formats,
    }
}

fn track_state_from_dir(
    config: &Config,
    album_dir: &Path,
    title: &str,
    track_num: usize,
) -> LocalTrackState {
    let mut formats = BTreeMap::new();
    let mut paths = Vec::new();

    for format in &config.download.formats {
        if format == "gift" {
            continue;
        }
        let ext = extension_for_format(format);
        let file_name = format!("{} {}.{}", track_num, sanitize_filename(title), ext);
        let path = album_dir.join(file_name);
        let exists = path.exists();
        if exists {
            paths.push(path.display().to_string());
        }
        formats.insert(format.clone(), exists);
    }

    let downloaded = !formats.is_empty() && formats.values().any(|exists| *exists);

    LocalTrackState {
        downloaded,
        formats,
        paths,
    }
}

fn build_album_index(output_dir: &Path) -> BTreeMap<String, PathBuf> {
    let mut index = BTreeMap::new();
    collect_album_index(output_dir, 0, &mut index);
    index
}

fn collect_album_index(dir: &Path, depth: usize, index: &mut BTreeMap<String, PathBuf>) {
    if depth > 4 || index.len() > 10_000 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Some(id) = read_album_id_from_metadata(&path) {
            index.entry(id).or_insert(path.clone());
        }
        collect_album_index(&path, depth + 1, index);
    }
}

fn read_album_id_from_metadata(dir: &Path) -> Option<String> {
    let nfo = dir.join("album.nfo");
    if let Ok(content) = fs::read_to_string(&nfo) {
        if let Some(id) = extract_between(&content, "<id>", "</id>") {
            return Some(id);
        }
    }

    let readme = dir.join("README.md");
    if let Ok(content) = fs::read_to_string(&readme) {
        for line in content.lines() {
            if line.contains("专辑ID") || line.to_ascii_lowercase().contains("album id") {
                if let Some((_, value)) = line.split_once(':') {
                    let id = value.trim().trim_matches('*').trim();
                    if !id.is_empty() {
                        return Some(id.to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_between(content: &str, start: &str, end: &str) -> Option<String> {
    let start_index = content.find(start)? + start.len();
    let tail = &content[start_index..];
    let end_index = tail.find(end)?;
    let value = tail[..end_index].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn album_directory_for_list_item(config: &Config, album: &DiscListItem) -> PathBuf {
    let current_year = chrono::Utc::now().year().to_string();
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let directory_name = config
        .paths
        .directory_template
        .replace("{album}", &sanitize_filename(&album.title))
        .replace("{label}", &sanitize_filename(&album.label))
        .replace("{authors}", &sanitize_filename(&album.label))
        .replace("{year}", &current_year)
        .replace("{date}", &current_date);
    config.paths.output_dir.join(directory_name)
}

fn album_directory_for_disc(config: &Config, album: &DiscInfo) -> PathBuf {
    let current_year = chrono::Utc::now().year().to_string();
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let authors = album
        .tracks
        .first()
        .map(|track| track.authers.as_str())
        .unwrap_or(&album.label);
    let year = album
        .release_date
        .as_deref()
        .and_then(metadata::extract_year_from_date)
        .unwrap_or(current_year);
    let date = album
        .release_date
        .as_ref()
        .map(|date| metadata::normalize_date(date))
        .unwrap_or(current_date);
    let directory_name = config
        .paths
        .directory_template
        .replace("{album}", &sanitize_filename(&album.title))
        .replace("{label}", &sanitize_filename(&album.label))
        .replace("{authors}", &sanitize_filename(authors))
        .replace("{year}", &year)
        .replace("{date}", &date);
    config.paths.output_dir.join(directory_name)
}

fn extension_for_format(format: &str) -> &str {
    match format {
        "128" | "320" => "mp3",
        "FLAC" => "flac",
        _ => "bin",
    }
}

fn count_extension(dir: &Path, extension: &str) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
        })
        .count()
}

fn count_audio_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "mp3" | "flac"))
                .unwrap_or(false)
        })
        .count()
}

fn has_cover(dir: &Path) -> bool {
    ["jpg", "png", "webp", "gif"]
        .iter()
        .any(|ext| dir.join(format!("cover.{ext}")).exists())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
