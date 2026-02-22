mod api;
mod auth;
mod web;

use anyhow::Result;
use reqwest::Client;
use tracing::debug;

#[derive(Clone)]
pub struct DizzylabClient {
    pub(super) client: Client,
    pub(super) debug: bool,
}

impl DizzylabClient {
    pub fn new(debug: bool) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_store(true)
            .build()?;

        Ok(Self { client, debug })
    }

    /// Download raw bytes from a CDN URL (no special headers needed)
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        debug!("下载: {}", url);
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("下载失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Download an archive using the web session cookie + Referer header
    pub async fn download_file(&self, url: &str, album_id: &str) -> Result<Vec<u8>> {
        use anyhow::anyhow;

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

        if !response.status().is_success() {
            return Err(anyhow!("下载失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn download_cover(&self, cover_url: &str, album_id: &str) -> Result<Vec<u8>> {
        use anyhow::anyhow;

        if cover_url.is_empty() {
            return Err(anyhow!("封面URL为空"));
        }

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

    pub(super) async fn log_response_text(
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
