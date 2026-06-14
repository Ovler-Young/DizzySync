use super::DizzylabClient;
use anyhow::{anyhow, Result};
use tracing::{debug, info};

impl DizzylabClient {
    /// Login via both web session (for gift/web downloads) and API token.
    /// Returns the API token for use in all JSON API calls.
    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        info!("登录中...");

        // Step 1: GET login page to obtain csrftoken cookie
        let login_page_url = "https://www.dizzylab.net/albums/login/";
        let response = self.client.get(login_page_url).send().await?;
        let login_page_status = response.status();

        if self.debug {
            debug!("登录页面状态码: {}", login_page_status);
        }
        if !login_page_status.is_success() {
            return Err(anyhow!("获取登录页面失败，HTTP {}", login_page_status));
        }

        let csrf_token = self.extract_csrftoken_from_response(&response)?;
        debug!("获取到 csrftoken");

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

        let web_login_status = web_login_resp.status();
        if self.debug {
            debug!("网页登录响应状态码: {}", web_login_status);
        }
        if !(web_login_status.is_success() || web_login_status.is_redirection()) {
            return Err(anyhow!("网页登录失败，HTTP {}", web_login_status));
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

        let api_status = api_resp.status();
        let api_resp_text = api_resp.text().await?;
        if !api_status.is_success() {
            return Err(anyhow!(
                "API 登录失败，HTTP {}: {}",
                api_status,
                &api_resp_text[..api_resp_text.len().min(200)]
            ));
        }
        if self.debug {
            debug!(
                "API 登录响应: {}",
                super::redact_text_for_log(&api_resp_text)
            );
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
                    let token = token_part.split(';').next().unwrap_or("").trim();
                    if !token.is_empty() {
                        return Ok(token.to_string());
                    }
                }
            }
        }
        Err(anyhow!("无法从响应头中获取 csrftoken"))
    }
}
