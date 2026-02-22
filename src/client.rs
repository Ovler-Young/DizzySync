use anyhow::{anyhow, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

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

#[derive(Debug, Deserialize)]
struct MyInfoResponse {
    user: MyInfoUser,
}

#[derive(Debug, Deserialize)]
struct MyInfoUser {
    uid: serde_json::Value,
    username: String,
}

#[derive(Debug, Deserialize)]
struct MyDiscResponse {
    discs: Vec<DiscListItem>,
    #[serde(default)]
    canshowmore: bool,
}

#[derive(Debug, Deserialize)]
struct TrackDownloadResponse {
    track: TrackDownloadInfo,
}

#[derive(Debug, Deserialize)]
struct TrackDownloadInfo {
    url: String,
}

#[derive(Clone)]
pub struct DizzylabClient {
    client: Client,
    debug: bool,
}

impl DizzylabClient {
    pub fn new(debug: bool) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_store(true)
            .build()?;

        Ok(Self { client, debug })
    }

    /// Login via both web session (for gift downloads) and API token.
    /// Returns the API token for use in all JSON API calls.
    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        info!("登录中...");

        // Step 1: GET login page to obtain csrftoken cookie
        let login_page_url = "https://www.dizzylab.net/albums/login/";
        let response = self.client.get(login_page_url).send().await?;

        // Extract csrftoken from Set-Cookie headers
        let csrf_token = self.extract_csrftoken_from_response(&response)?;
        debug!("获取到 csrftoken");

        if self.debug {
            debug!("登录页面状态码: {}", response.status());
        }

        // Step 2: POST web login form to establish session cookies
        let form_params = [
            ("csrfmiddlewaretoken", csrf_token.as_str()),
            ("next", ""),
            ("username", username),
            ("password", password),
        ];

        let web_login_resp = self
            .client
            .post(login_page_url)
            .header("Referer", login_page_url)
            .form(&form_params)
            .send()
            .await?;

        if self.debug {
            debug!("网页登录响应状态码: {}", web_login_resp.status());
        }

        info!("网页会话已建立");

        // Step 3: POST to mobile API to get token
        let api_login_body = serde_json::json!({
            "username": username,
            "password": password,
        });

        let api_resp = self
            .client
            .post("https://www.dizzylab.net/apis/auth/login/")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&api_login_body)
            .send()
            .await?;

        let api_resp_text = api_resp.text().await?;
        if self.debug {
            debug!("API 登录响应: {}", api_resp_text);
        }

        let api_resp_json: serde_json::Value = serde_json::from_str(&api_resp_text)?;
        let token = api_resp_json
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("登录失败：响应中没有 token，请检查用户名和密码"))?
            .to_string();

        info!("登录成功");
        Ok(token)
    }

    fn extract_csrftoken_from_response(&self, response: &reqwest::Response) -> Result<String> {
        for (name, value) in response.headers() {
            if name.as_str().eq_ignore_ascii_case("set-cookie") {
                let cookie_str = value.to_str().unwrap_or("");
                if let Some(token_part) = cookie_str.strip_prefix("csrftoken=") {
                    // Format: "csrftoken=VALUE; ..."
                    let token = token_part.split(';').next().unwrap_or("").trim();
                    if !token.is_empty() {
                        return Ok(token.to_string());
                    }
                }
            }
        }
        Err(anyhow!("无法从响应头中获取 csrftoken"))
    }

    pub async fn get_my_info(&self, token: &str) -> Result<UserInfo> {
        info!("获取用户信息...");

        let url = format!("https://www.dizzylab.net/apis/getmyinfo/?token={token}");
        let response = self.client.get(&url).send().await?;
        let text = self.log_response_text(response, "getmyinfo").await?;

        let parsed: MyInfoResponse = serde_json::from_str(&text)?;
        let uid = match &parsed.user.uid {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };

        info!("用户: {} (UID: {})", parsed.user.username, uid);
        Ok(UserInfo {
            uid,
            username: parsed.user.username,
        })
    }

    pub async fn get_my_discs(&self, token: &str) -> Result<Vec<DiscListItem>> {
        info!("获取已购专辑列表...");

        let mut all_discs = Vec::new();
        let mut offset = 0u32;
        const PAGE_SIZE: u32 = 9;

        loop {
            let url = format!(
                "https://www.dizzylab.net/apis/getmydisc/?l={}&r={}&sort=ad&token={}",
                offset,
                offset + PAGE_SIZE,
                token
            );
            debug!("请求专辑列表: {}", url);

            let response = self.client.get(&url).send().await?;
            let text = self
                .log_response_text(response, &format!("getmydisc offset={offset}"))
                .await?;

            let parsed: MyDiscResponse = serde_json::from_str(&text)?;
            let can_show_more = parsed.canshowmore;
            all_discs.extend(parsed.discs);

            if !can_show_more {
                break;
            }

            offset += PAGE_SIZE;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("获取到 {} 个专辑", all_discs.len());
        Ok(all_discs)
    }

    pub async fn get_disc_info(&self, discid: &str, token: &str) -> Result<DiscInfo> {
        info!("获取专辑详情: {}", discid);

        let url =
            format!("https://www.dizzylab.net/apis/getthisdicsinfo/?discid={discid}&token={token}");
        let response = self.client.get(&url).send().await?;
        let text = self
            .log_response_text(response, &format!("getthisdicsinfo {discid}"))
            .await?;

        let disc_info: DiscInfo = serde_json::from_str(&text)?;
        Ok(disc_info)
    }

    pub async fn get_track_download_url(
        &self,
        discid: &str,
        trackid: &str,
        packtype: &str,
        token: &str,
    ) -> Result<String> {
        let url = format!(
            "https://www.dizzylab.net/apis/gettrackdownloadurl/?discid={discid}&trackid={trackid}&packtype={packtype}&token={token}"
        );
        debug!(
            "获取曲目下载链接: discid={} trackid={} packtype={}",
            discid, trackid, packtype
        );

        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = self
            .log_response_text(response, &format!("gettrackdownloadurl {trackid}"))
            .await?;

        if !status.is_success() {
            return Err(anyhow!(
                "获取曲目下载链接失败，HTTP {} (trackid={}, packtype={}): {}",
                status,
                trackid,
                packtype,
                &text[..text.len().min(200)]
            ));
        }

        let parsed: TrackDownloadResponse = serde_json::from_str(&text).map_err(|e| {
            anyhow!(
                "解析曲目下载URL失败 (trackid={}, packtype={}): {} | 响应: {:?}",
                trackid,
                packtype,
                e,
                &text[..text.len().min(200)]
            )
        })?;

        Ok(parsed.track.url)
    }

    /// Download raw bytes from a CDN URL (no special headers needed)
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        debug!("下载: {}", url);
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("下载失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Download a gift archive using the web session cookie + Referer header
    pub async fn download_file(&self, url: &str, album_id: &str) -> Result<Vec<u8>> {
        info!("开始下载 gift: {}", album_id);

        let response = self
            .client
            .get(url)
            .header(
                "Referer",
                &format!("https://www.dizzylab.net/d/{album_id}/"),
            )
            .send()
            .await?;

        if self.debug {
            debug!("下载响应状态码: {} ({})", response.status(), album_id);
        }

        if response.status().is_redirection() {
            if let Some(location) = response.headers().get("location") {
                let redirect_url = location.to_str()?;
                debug!("重定向到: {}", redirect_url);
                return self.download_bytes(redirect_url).await;
            }
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

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

    pub async fn download_cover(&self, cover_url: &str, album_id: &str) -> Result<Vec<u8>> {
        if cover_url.is_empty() {
            return Err(anyhow!("封面URL为空"));
        }

        info!("下载封面: {} (专辑: {})", cover_url, album_id);

        let response = self
            .client
            .get(cover_url)
            .header(
                "Referer",
                &format!("https://www.dizzylab.net/d/{album_id}/"),
            )
            .send()
            .await?;

        if self.debug {
            debug!("封面下载状态码: {} ({})", response.status(), album_id);
        }

        if !response.status().is_success() {
            return Err(anyhow!("下载封面失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn log_response_text(
        &self,
        response: reqwest::Response,
        context: &str,
    ) -> Result<String> {
        let status = response.status();
        let text = response.text().await?;

        if self.debug {
            debug!("=== HTTP [{context}] status={status} body={text} ===");
        }

        Ok(text)
    }
}
