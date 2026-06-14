use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAlbumState {
    /// True only when all configured downloadable formats are complete for the known track count.
    pub downloaded: bool,
    pub directory_exists: bool,
    pub path: String,
    pub audio_files: usize,
    pub expected_tracks: usize,
    pub downloaded_tracks: usize,
    #[serde(default)]
    pub complete_tracks: usize,
    #[serde(default)]
    pub has_media: bool,
    #[serde(default)]
    pub complete: bool,
    pub gift_exists: bool,
    pub formats: BTreeMap<String, bool>,
    /// Configured album-level formats that are not present locally (including gift).
    #[serde(default)]
    pub missing_formats: Vec<String>,
    /// Known tracks that are incomplete or missing locally. Populated when full album metadata is available.
    #[serde(default)]
    pub missing_tracks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTrackState {
    /// True only when all configured audio formats for this track exist.
    pub downloaded: bool,
    #[serde(default)]
    pub has_media: bool,
    #[serde(default)]
    pub complete: bool,
    pub formats: BTreeMap<String, bool>,
    pub paths: Vec<String>,
    /// Configured audio formats that are missing for this track.
    #[serde(default)]
    pub missing_formats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: String,
    pub username: String,
}

/// Disc item returned by getmydisc/ (owned disc list)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscListItem {
    #[serde(deserialize_with = "string_from_flexible_value")]
    pub id: String,
    #[serde(
        default,
        alias = "name",
        deserialize_with = "string_from_optional_value"
    )]
    pub title: String,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub label: String,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub cover: String,
    #[serde(default, alias = "label_id")]
    pub labelid: Option<serde_json::Value>,
    #[serde(
        default,
        alias = "releaseDate",
        alias = "release_date_str",
        alias = "releaseDateStr",
        alias = "releasedate",
        alias = "release",
        alias = "publish_date",
        alias = "publishDate",
        alias = "date",
        deserialize_with = "optional_string_from_flexible_value"
    )]
    pub release_date: Option<String>,
    #[serde(default)]
    pub price: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub hasgift: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub ispreselling: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub onsell: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub onlyhavegift: bool,
    #[serde(default, deserialize_with = "string_vec_from_flexible_value")]
    pub tags: Vec<String>,
    #[serde(
        default,
        alias = "trackCount",
        alias = "trackcount",
        alias = "tracks_count",
        alias = "trackNum",
        alias = "track_num",
        alias = "song_count",
        alias = "songs_count",
        alias = "music_count",
        deserialize_with = "optional_usize_from_flexible_value"
    )]
    pub track_count: Option<usize>,
    #[serde(
        default,
        alias = "format",
        alias = "download_formats",
        alias = "downloadFormats",
        deserialize_with = "string_vec_from_flexible_value"
    )]
    pub formats: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalAlbumState>,
}

/// Full disc info returned by getthisdicsinfo/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscInfo {
    #[serde(deserialize_with = "string_from_flexible_value")]
    pub id: String,
    #[serde(
        default,
        alias = "name",
        deserialize_with = "string_from_optional_value"
    )]
    pub title: String,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub label: String,
    #[serde(default, alias = "label_id")]
    pub labelid: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub cover: String,
    #[serde(default)]
    pub labelcover: Option<String>,
    #[serde(default)]
    pub label_description: Option<String>,
    #[serde(default)]
    pub disc_description: Option<String>,
    #[serde(default)]
    pub disc_description_2: Option<String>,
    #[serde(
        default,
        alias = "releaseDate",
        alias = "release_date_str",
        alias = "releaseDateStr",
        alias = "releasedate",
        alias = "release",
        alias = "publish_date",
        alias = "publishDate",
        alias = "date",
        deserialize_with = "optional_string_from_flexible_value"
    )]
    pub release_date: Option<String>,
    #[serde(default)]
    pub price: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub hasgift: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub ispreselling: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub onsell: bool,
    #[serde(default, deserialize_with = "bool_from_flexible_value")]
    pub onlyhavegift: bool,
    #[serde(default, deserialize_with = "string_vec_from_flexible_value")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalAlbumState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    #[serde(deserialize_with = "string_from_flexible_value")]
    pub id: String,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub discid: String,
    #[serde(
        default,
        alias = "name",
        deserialize_with = "string_from_optional_value"
    )]
    pub title: String,
    #[serde(default, deserialize_with = "optional_string_from_flexible_value")]
    pub album: Option<String>,
    #[serde(
        default,
        alias = "authors",
        alias = "artists",
        alias = "artist",
        deserialize_with = "string_from_optional_value"
    )]
    pub authers: String, // note: API typo preserved
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub label: String,
    #[serde(default, deserialize_with = "string_from_optional_value")]
    pub url: String,
    #[serde(
        default,
        alias = "cover",
        deserialize_with = "string_from_optional_value"
    )]
    pub coverurl: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalTrackState>,
}

fn optional_string_from_value(value: serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Array(values) => values.into_iter().find_map(optional_string_from_value),
        serde_json::Value::Object(_) => None,
    }
}

fn string_from_flexible_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(optional_string_from_value(value).unwrap_or_default())
}

fn string_from_optional_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value
        .and_then(optional_string_from_value)
        .unwrap_or_default())
}

fn optional_string_from_flexible_value<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(optional_string_from_value))
}

fn optional_usize_from_flexible_value<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        serde_json::Value::Number(n) => n.as_u64().map(|n| n as usize),
        serde_json::Value::String(s) => s.trim().parse::<usize>().ok(),
        serde_json::Value::Array(values) => Some(values.len()),
        _ => None,
    }))
}

fn bool_from_flexible_value<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Bool(b)) => b,
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or_default() != 0,
        Some(serde_json::Value::String(s)) => {
            matches!(
                s.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y"
            )
        }
        _ => false,
    })
}

fn string_vec_from_flexible_value<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let mut out = Vec::new();
    match value {
        Some(serde_json::Value::Array(values)) => {
            for value in values {
                if let Some(s) = optional_string_from_value(value) {
                    if !s.is_empty() {
                        out.push(s);
                    }
                }
            }
        }
        Some(value) => {
            if let Some(s) = optional_string_from_value(value) {
                out.extend(
                    s.split([',', '/', '|'])
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(ToOwned::to_owned),
                );
            }
        }
        None => {}
    }
    Ok(out)
}
