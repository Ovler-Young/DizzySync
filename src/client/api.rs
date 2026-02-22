use super::DizzylabClient;
use crate::types::{DiscInfo, DiscListItem, UserInfo};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use tracing::{debug, info};

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

impl DizzylabClient {
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
}
