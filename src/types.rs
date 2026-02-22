use serde::{Deserialize, Serialize};

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
    pub disc_description: Option<String>,
    #[serde(default)]
    pub disc_description_2: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub hasgift: bool,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub discid: String,
    pub title: String,
    #[serde(default)]
    pub authers: String, // note: API typo preserved
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub coverurl: String,
}
