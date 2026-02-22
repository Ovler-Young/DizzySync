use super::DizzylabClient;
use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;
use tracing::info;

impl DizzylabClient {
    /// Scrape the album page to extract the gift download key.
    /// Returns an empty map if the album has no gift content.
    pub async fn get_gift_download_link(&self, album_id: &str) -> Result<HashMap<String, String>> {
        let album_url = format!("https://www.dizzylab.net/d/{album_id}/");
        let response = self.client.get(&album_url).send().await?;

        let html = self
            .log_response_text(response, &format!("gift key {album_id}"))
            .await?;
        let document = Html::parse_document(&html);

        let key_regex = regex::Regex::new(r"k=([^&]+)")?;
        let selector = Selector::parse(r#"a[href*="/albums/download_gift/"]"#).unwrap();

        if let Some(element) = document.select(&selector).next() {
            if let Some(href) = element.value().attr("href") {
                if let Some(captures) = key_regex.captures(href) {
                    if let Some(key) = captures.get(1) {
                        let download_url = format!(
                            "https://www.dizzylab.net/albums/download_gift/{album_id}/?k={}",
                            key.as_str()
                        );
                        let mut result = HashMap::new();
                        result.insert("gift".to_string(), download_url);
                        return Ok(result);
                    }
                }
            }
        }

        info!("专辑 {} 没有特典内容，跳过", album_id);
        Ok(HashMap::new())
    }

    /// Scrape the album page to get the ZIP/RAR download link for a non-gift format (128/FLAC/etc).
    /// Uses the web session established during login.
    pub async fn get_web_format_download_link(
        &self,
        album_id: &str,
        format: &str,
    ) -> Result<String> {
        let album_url = format!("https://www.dizzylab.net/d/{album_id}/");
        let response = self.client.get(&album_url).send().await?;
        let html = self
            .log_response_text(response, &format!("web format link {album_id} {format}"))
            .await?;

        let document = Html::parse_document(&html);
        let key_regex = regex::Regex::new(r"k=([^&]+)")?;
        let selector = Selector::parse(&format!(r#"a[href*="tp={format}"]"#)).unwrap();

        if let Some(element) = document.select(&selector).next() {
            if let Some(href) = element.value().attr("href") {
                if let Some(captures) = key_regex.captures(href) {
                    if let Some(key) = captures.get(1) {
                        let download_url = format!(
                            "https://www.dizzylab.net/albums/download/?d={album_id}&tp={format}&k={}",
                            key.as_str()
                        );
                        return Ok(download_url);
                    }
                }
            }
        }

        Err(anyhow!(
            "无法从页面中找到格式 {} 的下载链接 (专辑: {})",
            format,
            album_id
        ))
    }
}
