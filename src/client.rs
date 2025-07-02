use anyhow::{anyhow, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: u32,
    pub allcount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub label: String,
    pub cover: String,
    #[serde(rename = "onlyhavegift")]
    pub only_have_gift: bool,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub year: Option<String>,
    #[serde(default)]
    pub authors: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumListResponse {
    pub user: UserInfo,
    pub discs: Vec<Album>,
    #[serde(rename = "canshowmore")]
    pub can_show_more: bool,
}

pub struct DizzylabClient {
    client: Client,
    cookie: String,
    debug: bool,
}

impl DizzylabClient {
    pub fn new(cookie: String, debug: bool) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        Ok(Self { client, cookie, debug })
    }

    // 辅助方法：记录HTTP响应用于调试
    async fn log_response(&self, response: reqwest::Response, context: &str) -> Result<String> {
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        
        if self.debug {
            debug!("=== HTTP 调试信息 ({}) ===", context);
            debug!("状态码: {}", status);
            debug!("响应头: {:#?}", headers);
            debug!("响应体: {}", text);
            debug!("=== HTTP 调试信息结束 ===");
        }
        
        Ok(text)
    }

    // 辅助方法：记录JSON响应用于调试
    async fn log_json_response<T>(&self, response: reqwest::Response, context: &str) -> Result<T> 
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        
        if self.debug {
            debug!("=== HTTP 调试信息 ({}) ===", context);
            debug!("状态码: {}", status);
            debug!("响应头: {:#?}", headers);
            debug!("响应体: {}", text);
            debug!("=== HTTP 调试信息结束 ===");
        }
        
        let result: T = serde_json::from_str(&text)?;
        Ok(result)
    }

    pub async fn get_user_info(&self) -> Result<UserInfo> {
        info!("获取用户信息...");
        
        let response = self
            .client
            .get("https://www.dizzylab.net/")
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        let html = self.log_response(response, "获取用户信息").await?;
        let document = Html::parse_document(&html);

        // 从HTML中提取用户ID
        let uid = self.extract_user_id(&document)?;

        info!("获取到用户信息: ID={}", uid);

        Ok(UserInfo {
            uid,
            allcount: 0, // 这个值在后续API调用中会更新
        })
    }

    pub async fn get_user_albums(&self, uid: u32) -> Result<Vec<Album>> {
        info!("获取用户专辑列表...");
        
        let mut all_albums = Vec::new();
        let mut offset = 0;
        const LIMIT: u32 = 20;

        // 首先需要获取token
        let token = self.get_user_token(uid).await?;

        loop {
            let url = format!(
                "https://www.dizzylab.net/apis/getotheruserinfo/?l={}&r={}&uid={}&token={}",
                offset,
                offset + LIMIT,
                uid,
                token
            );

            debug!("请求专辑列表: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Cookie", &self.cookie)
                .header("Referer", &format!("https://www.dizzylab.net/u/{}/music/", uid))
                .send()
                .await?;

            let album_response: AlbumListResponse = self.log_json_response(response, &format!("获取专辑列表 offset={}", offset)).await?;
            
            all_albums.extend(album_response.discs);
            
            if !album_response.can_show_more {
                break;
            }
            
            offset += LIMIT;
            
            // 添加延迟避免请求过快
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("获取到 {} 个专辑", all_albums.len());
        Ok(all_albums)
    }

    pub async fn get_album_by_id(&self, album_id: &str) -> Result<Album> {
        info!("根据ID获取专辑信息: {}", album_id);
        
        // 创建基本的专辑结构
        let mut album = Album {
            id: album_id.to_string(),
            title: "未知专辑".to_string(),
            label: "未知厂牌".to_string(),
            cover: String::new(),
            only_have_gift: false,
            release_date: None,
            description: None,
            tags: Vec::new(),
            year: None,
            authors: None,
        };

        // 获取专辑详细信息，更新所有字段
        self.get_album_details(&mut album).await?;
        
        Ok(album)
    }

    pub async fn get_album_details(&self, album: &mut Album) -> Result<()> {
        info!("获取专辑 {} 的详细信息", album.id);

        let album_url = format!("https://www.dizzylab.net/d/{}/", album.id);
        let response = self
            .client
            .get(&album_url)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        let html = self.log_response(response, &format!("获取专辑详情 {}", album.id)).await?;
        let document = Html::parse_document(&html);

        // 提取详细信息
        // 如果标题或厂牌是默认值，尝试从页面提取
        if album.title == "未知专辑" {
            album.title = self.extract_title(&document)?.unwrap_or_else(|| album.title.clone());
        }
        if album.label == "未知厂牌" {
            album.label = self.extract_label(&document)?.unwrap_or_else(|| album.label.clone());
        }
        album.release_date = self.extract_release_date(&document)?;
        album.description = self.extract_description(&document)?;
        album.tags = self.extract_tags(&document);
        album.year = self.extract_year(&document)?;
        album.authors = self.extract_authors(&document)?;

        Ok(())
    }

    pub async fn get_download_links(&self, album_id: &str, format: &str) -> Result<HashMap<String, String>> {
        info!("获取专辑 {} 的下载链接 (格式: {})", album_id, format);

        // 首先访问专辑页面获取下载密钥
        let album_url = format!("https://www.dizzylab.net/d/{}/", album_id);
        let response = self
            .client
            .get(&album_url)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        let html = self.log_response(response, &format!("获取下载密钥 {} {}", album_id, format)).await?;
        let document = Html::parse_document(&html);

        // 从HTML中提取下载密钥
        let download_key = match self.extract_download_key(&document, format) {
            Ok(key) => key,
            Err(_) => {
                // 如果是gift格式且找不到，说明该专辑没有特典内容，返回空结果
                if format == "gift" {
                    info!("专辑 {} 没有特典内容，跳过", album_id);
                    return Ok(HashMap::new());
                } else {
                    // 对于其他格式，仍然返回错误
                    return Err(anyhow!("无法从页面中提取下载密钥，格式: {}", format));
                }
            }
        };

        let download_url = if format == "gift" {
            format!(
                "https://www.dizzylab.net/albums/download_gift/{}/?k={}",
                album_id, download_key
            )
        } else {
            format!(
                "https://www.dizzylab.net/albums/download/?d={}&tp={}&k={}",
                album_id, format, download_key
            )
        };

        debug!("下载URL: {}", download_url);

        let mut result = HashMap::new();
        result.insert(format.to_string(), download_url);

        Ok(result)
    }

    pub async fn download_file(&self, url: &str, album_id: &str) -> Result<Vec<u8>> {
        info!("开始下载: {}", album_id);

        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookie)
            .header("Referer", &format!("https://www.dizzylab.net/d/{}/", album_id))
            .send()
            .await?;

        if self.debug {
            debug!("=== HTTP 下载调试信息 ({}) ===", album_id);
            debug!("下载URL: {}", url);
            debug!("状态码: {}", response.status());
            debug!("响应头: {:#?}", response.headers());
            debug!("=== HTTP 下载调试信息结束 ===");
        }

        // 检查是否是重定向响应
        if response.status().is_redirection() {
            if let Some(location) = response.headers().get("location") {
                let redirect_url = location.to_str()?;
                debug!("重定向到: {}", redirect_url);
                return self.download_from_cdn(redirect_url).await;
            }
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn download_from_cdn(&self, url: &str) -> Result<Vec<u8>> {
        if self.debug {
            debug!("=== CDN 下载调试信息 ===");
            debug!("CDN URL: {}", url);
        }
        
        let response = self.client.get(url).send().await?;
        
        if self.debug {
            debug!("CDN 状态码: {}", response.status());
            debug!("CDN 响应头: {:#?}", response.headers());
            debug!("=== CDN 下载调试信息结束 ===");
        }
        
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn get_user_token(&self, uid: u32) -> Result<String> {
        debug!("获取用户token...");
        
        let url = format!("https://www.dizzylab.net/u/{}/music/", uid);
        let response = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        let html = self.log_response(response, &format!("获取token uid={}", uid)).await?;
        
        // 从JavaScript代码中提取token
        if let Some(start) = html.find("token = '") {
            let start = start + 9;
            if let Some(end) = html[start..].find("'") {
                let token = &html[start..start + end];
                debug!("获取到token: {}", token);
                return Ok(token.to_string());
            }
        }

        Err(anyhow!("无法从页面中提取token"))
    }

    fn extract_user_id(&self, document: &Html) -> Result<u32> {
        // 查找包含用户ID的链接或元素
        let selector = Selector::parse(r#"a[href*="/u/"]"#).unwrap();
        
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if let Some(captures) = regex::Regex::new(r"/u/(\d+)")?.captures(href) {
                    if let Some(uid_str) = captures.get(1) {
                        return Ok(uid_str.as_str().parse()?);
                    }
                }
            }
        }

        Err(anyhow!("无法从页面中提取用户ID"))
    }

    fn extract_title(&self, document: &Html) -> Result<Option<String>> {
        // 从页面标题中提取专辑名称
        let title_selector = Selector::parse("title").unwrap();
        if let Some(title_element) = document.select(&title_selector).next() {
            let title_text = title_element.text().collect::<Vec<_>>().join("");
            // Dizzylab的页面标题格式通常是 "专辑名 - Dizzylab"
            if let Some(album_title) = title_text.split(" - ").next() {
                return Ok(Some(album_title.trim().to_string()));
            }
        }

        // 尝试从h1标签提取
        let h1_selector = Selector::parse("h1").unwrap();
        if let Some(h1_element) = document.select(&h1_selector).next() {
            let h1_text = h1_element.text().collect::<Vec<_>>().join("").trim().to_string();
            if !h1_text.is_empty() {
                return Ok(Some(h1_text));
            }
        }

        Ok(None)
    }

    fn extract_label(&self, document: &Html) -> Result<Option<String>> {
        // 尝试从页面中提取厂牌信息
        // 这可能需要根据实际的Dizzylab页面结构来调整
        let label_selector = Selector::parse(".album-info .label, .disc-label, [class*='label']").unwrap();
        if let Some(label_element) = document.select(&label_selector).next() {
            let label_text = label_element.text().collect::<Vec<_>>().join("").trim().to_string();
            if !label_text.is_empty() {
                return Ok(Some(label_text));
            }
        }

        // 如果找不到，尝试从链接中提取
        let link_selector = Selector::parse(r#"a[href*="/l/"]"#).unwrap();
        if let Some(link_element) = document.select(&link_selector).next() {
            let link_text = link_element.text().collect::<Vec<_>>().join("").trim().to_string();
            if !link_text.is_empty() {
                return Ok(Some(link_text));
            }
        }

        Ok(None)
    }

    fn extract_download_key(&self, document: &Html, format: &str) -> Result<String> {
        // 从下载链接中提取密钥
        if format == "gift" {
            // gift: /albums/download_gift/ALBUM_ID/?k=KEY
            let selector = Selector::parse(r#"a[href*="/albums/download_gift/"]"#).unwrap();
            
            if let Some(element) = document.select(&selector).next() {
                if let Some(href) = element.value().attr("href") {
                    if let Some(captures) = regex::Regex::new(r"k=([^&]+)")?.captures(href) {
                        if let Some(key) = captures.get(1) {
                            return Ok(key.as_str().to_string());
                        }
                    }
                }
            }
        } else {
            let tp_param = match format {
                "128" => "128",
                "MP3" => "MP3", 
                "FLAC" => "FLAC",
                _ => format,
            };

            let selector = Selector::parse(&format!(r#"a[href*="tp={}"]"#, tp_param)).unwrap();
            
            if let Some(element) = document.select(&selector).next() {
                if let Some(href) = element.value().attr("href") {
                    if let Some(captures) = regex::Regex::new(r"k=([^&]+)")?.captures(href) {
                        if let Some(key) = captures.get(1) {
                            return Ok(key.as_str().to_string());
                        }
                    }
                }
            }
        }

        Err(anyhow!("无法从页面中提取下载密钥，格式: {}", format))
    }

    fn extract_release_date(&self, document: &Html) -> Result<Option<String>> {
        // 查找发布日期，通常在页面下方
        let selector = Selector::parse("p").unwrap();
        
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            if text.contains("发布于") {
                // 提取日期部分，如 "发布于2025年6月10日"
                if let Some(captures) = regex::Regex::new(r"发布于(\d{4}年\d{1,2}月\d{1,2}日)")?.captures(&text) {
                    if let Some(date) = captures.get(1) {
                        return Ok(Some(date.as_str().to_string()));
                    }
                }
            }
        }
        
        Ok(None)
    }

    fn extract_description(&self, document: &Html) -> Result<Option<String>> {
        // 查找专辑描述，通常在页面中的某个段落中
        let selector = Selector::parse("h3").unwrap();
        
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join("").trim().to_string();
            if !text.is_empty() && text.len() > 20 {
                // 过滤掉太短的文本，可能是标题
                return Ok(Some(text));
            }
        }
        
        Ok(None)
    }

    fn extract_tags(&self, document: &Html) -> Vec<String> {
        let mut tags = Vec::new();
        let selector = Selector::parse("a[href*='/albums/tags/']").unwrap();
        
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            if text.starts_with('#') {
                tags.push(text[1..].to_string()); // 移除#前缀
            }
        }
        
        tags
    }

    fn extract_year(&self, document: &Html) -> Result<Option<String>> {
        // 从发布日期中提取年份
        if let Some(date) = self.extract_release_date(document)? {
            if let Some(captures) = regex::Regex::new(r"(\d{4})年")?.captures(&date) {
                if let Some(year) = captures.get(1) {
                    return Ok(Some(year.as_str().to_string()));
                }
            }
        }
        
        Ok(None)
    }

    fn extract_authors(&self, document: &Html) -> Result<Option<String>> {
        // 尝试从描述中提取作者信息
        if let Some(description) = self.extract_description(document)? {
            // 查找包含作者信息的模式，如 "music：作者名" 或 "作词：作者名"
            let patterns = [
                r"music[：:]\s*([^\n\r<]+)",
                r"作曲[：:]\s*([^\n\r<]+)",
                r"作词[：:]\s*([^\n\r<]+)",
                r"Lyrics[：:]\s*([^\n\r<]+)",
            ];
            
            for pattern in &patterns {
                if let Some(captures) = regex::Regex::new(pattern)?.captures(&description) {
                    if let Some(author) = captures.get(1) {
                        return Ok(Some(author.as_str().trim().to_string()));
                    }
                }
            }
        }
        
        Ok(None)
    }

    pub async fn download_cover(&self, cover_url: &str, album_id: &str) -> Result<Vec<u8>> {
        if cover_url.is_empty() {
            return Err(anyhow!("封面URL为空"));
        }

        info!("下载封面: {} (专辑: {})", cover_url, album_id);

        let response = self
            .client
            .get(cover_url)
            .header("Referer", &format!("https://www.dizzylab.net/d/{}/", album_id))
            .send()
            .await?;

        if self.debug {
            debug!("=== HTTP 封面下载调试信息 ({}) ===", album_id);
            debug!("封面URL: {}", cover_url);
            debug!("状态码: {}", response.status());
            debug!("响应头: {:#?}", response.headers());
            debug!("=== HTTP 封面下载调试信息结束 ===");
        }

        if !response.status().is_success() {
            return Err(anyhow!("下载封面失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
} 