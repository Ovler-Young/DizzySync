use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAlbumState {
    pub downloaded: bool,
    pub directory_exists: bool,
    pub path: String,
    pub audio_files: usize,
    pub expected_tracks: usize,
    pub downloaded_tracks: usize,
    pub gift_exists: bool,
    pub formats: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTrackState {
    pub downloaded: bool,
    pub formats: BTreeMap<String, bool>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: String,
    pub username: String,
}

/// Disc item returned by getmydisc/ (owned disc list)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscListItem {
    pub id: String,
    pub title: String,
    pub label: String,
    pub cover: String,
    #[serde(default)]
    pub labelid: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalAlbumState>,
}

/// Full disc info returned by getthisdicsinfo/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscInfo {
    pub id: String,
    pub title: String,
    pub label: String,
    #[serde(default)]
    pub labelid: Option<serde_json::Value>,
    pub cover: String,
    #[serde(default)]
    pub labelcover: Option<String>,
    #[serde(default)]
    pub label_description: Option<String>,
    #[serde(default)]
    pub disc_description: Option<String>,
    #[serde(default)]
    pub disc_description_2: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub price: Option<serde_json::Value>,
    #[serde(default)]
    pub hasgift: bool,
    #[serde(default)]
    pub ispreselling: bool,
    #[serde(default)]
    pub onsell: bool,
    #[serde(default)]
    pub onlyhavegift: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalAlbumState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub discid: String,
    pub title: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub authers: String, // note: API typo preserved
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub coverurl: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<LocalTrackState>,
}
