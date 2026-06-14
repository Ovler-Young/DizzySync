use crate::client::DizzylabClient;
use crate::config::{Config, UserConfig};
use crate::downloader::Downloader;
use crate::types::{DiscInfo, DiscListItem, UserInfo};
use anyhow::{anyhow, Result};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct ApiServerOptions {
    pub config_path: String,
    pub config: Config,
}

#[derive(Clone)]
struct ApiState {
    sessions: Arc<RwLock<Vec<AccountSession>>>,
    config: Arc<RwLock<Config>>,
    config_path: String,
    job: Arc<Mutex<JobState>>,
    last_error: Arc<RwLock<Option<String>>>,
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

#[derive(Clone)]
struct AccountSession {
    account: UserConfig,
    client: DizzylabClient,
    token: String,
    user: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum JobState {
    Idle,
    Running { kind: String, started_at: u64 },
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    status: &'static str,
    ready: bool,
    configured: bool,
    requires_auth: bool,
    user: Option<UserInfo>,
    users: Vec<UserInfo>,
    job: JobState,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    timestamp: u64,
    level: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
struct ConfigResponse {
    config_path: String,
    exists: bool,
    config: PublicConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicConfig {
    user: PublicUserConfig,
    users: Vec<PublicUserConfig>,
    download: PublicDownloadConfig,
    paths: PublicPathsConfig,
    behavior: PublicBehaviorConfig,
    api: PublicApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PublicUserConfig {
    username: String,
    has_password: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicDownloadConfig {
    formats: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicPathsConfig {
    output_dir: String,
    directory_template: String,
    output_dir_locked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicBehaviorConfig {
    skip_existing: bool,
    single_threaded: bool,
    max_concurrent_albums: usize,
    max_concurrent_albums_locked: bool,
    generate_readme: bool,
    generate_nfo: bool,
    debug: bool,
    metadata_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicApiConfig {
    bind: String,
    has_api_key: bool,
    web_root: String,
}

#[derive(Debug, Deserialize)]
struct UpdateConfigRequest {
    user: Option<UpdateUserConfig>,
    users: Option<Vec<UpdateUserConfig>>,
    download: Option<UpdateDownloadConfig>,
    paths: Option<UpdatePathsConfig>,
    behavior: Option<UpdateBehaviorConfig>,
    api: Option<UpdateApiConfig>,
}

#[derive(Debug, Deserialize)]
struct UpdateUserConfig {
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateDownloadConfig {
    formats: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UpdatePathsConfig {
    output_dir: Option<String>,
    directory_template: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateBehaviorConfig {
    skip_existing: Option<bool>,
    single_threaded: Option<bool>,
    max_concurrent_albums: Option<usize>,
    generate_readme: Option<bool>,
    generate_nfo: Option<bool>,
    debug: Option<bool>,
    metadata_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateApiConfig {
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapConfigRequest {
    force: Option<bool>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyncRequest {
    id: Option<String>,
}

pub async fn run(options: ApiServerOptions) -> Result<()> {
    let mut config = options.config.clone();
    ensure_api_key_for_remote_bind(&mut config)?;
    config.save_to_file(&options.config_path)?;

    let state = ApiState {
        sessions: Arc::new(RwLock::new(Vec::new())),
        config: Arc::new(RwLock::new(config.clone())),
        config_path: options.config_path,
        job: Arc::new(Mutex::new(JobState::Idle)),
        last_error: Arc::new(RwLock::new(None)),
        logs: Arc::new(Mutex::new(Vec::new())),
    };
    push_log(&state, "info", "API/Web 控制服务初始化完成").await;

    if let Err(e) = ensure_logged_in(&state).await {
        error!("API 服务启动时登录失败: {}", e);
        *state.last_error.write().await = Some(e.to_string());
    }

    let bind = config.api.bind.clone();
    let web_root = config.api.web_root.clone();
    let api = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/logs", get(get_logs))
        .route("/config", get(get_config).put(update_config))
        .route("/config/bootstrap", post(bootstrap_config))
        .route("/albums", get(list_albums))
        .route("/albums/{id}", get(get_album))
        .route("/sync", post(start_sync))
        .route("/sync/{id}", post(start_album_sync))
        .with_state(state);

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(static_service(web_root));

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!("API/Web 控制服务已启动: http://{}", bind);
    axum::serve(listener, app).await?;
    Ok(())
}

fn static_service(web_root: PathBuf) -> ServeDir<ServeFile> {
    let index = web_root.join("index.html");
    ServeDir::new(web_root).fallback(ServeFile::new(index))
}

async fn health() -> Json<MessageResponse> {
    Json(MessageResponse {
        message: "ok".to_string(),
    })
}

async fn status(State(state): State<ApiState>) -> Json<StatusResponse> {
    let config = state.config.read().await.clone();
    let sessions = state.sessions.read().await.clone();
    let users = sessions
        .iter()
        .filter_map(|session| session.user.clone())
        .collect::<Vec<_>>();
    Json(StatusResponse {
        status: "ok",
        ready: !sessions.is_empty(),
        configured: has_credentials(&config),
        requires_auth: !config.api.api_key.is_empty(),
        user: users.first().cloned(),
        users,
        job: state.job.lock().await.clone(),
        last_error: state.last_error.read().await.clone(),
    })
}

async fn get_logs(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LogEntry>>, ApiError> {
    authorize(&state, &headers).await?;
    Ok(Json(state.logs.lock().await.clone()))
}

async fn get_config(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<ConfigResponse>, ApiError> {
    authorize(&state, &headers).await?;
    let config = state.config.read().await.clone();
    Ok(Json(ConfigResponse {
        config_path: state.config_path.clone(),
        exists: std::path::Path::new(&state.config_path).exists(),
        config: PublicConfig::from_config(&config),
    }))
}

async fn update_config(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>, ApiError> {
    authorize(&state, &headers).await?;

    let mut next_config = state.config.read().await.clone();
    apply_config_update(&mut next_config, req);
    next_config.apply_env_overrides(true);
    validate_credentials(&next_config).map_err(ApiError::bad_request)?;
    validate_formats(&next_config).map_err(ApiError::bad_request)?;

    // Validate all credentials before committing the config to memory or disk.
    let next_sessions = login_accounts(&next_config)
        .await
        .map_err(|e| ApiError::unauthorized(format!("登录失败: {e}")))?;

    next_config.save_to_file(&state.config_path)?;
    *state.config.write().await = next_config;
    *state.sessions.write().await = next_sessions;
    push_log(&state, "info", "配置已验证并保存").await;

    let config = state.config.read().await.clone();
    Ok(Json(ConfigResponse {
        config_path: state.config_path.clone(),
        exists: std::path::Path::new(&state.config_path).exists(),
        config: PublicConfig::from_config(&config),
    }))
}

async fn bootstrap_config(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(req): Json<BootstrapConfigRequest>,
) -> Result<Json<ConfigResponse>, ApiError> {
    authorize(&state, &headers).await?;

    let exists = std::path::Path::new(&state.config_path).exists();
    let force = req.force == Some(true);
    if exists && !force {
        return Err(ApiError::conflict(
            "配置文件已存在；如需覆盖请设置 force=true",
        ));
    }
    if force && state.config.read().await.api.api_key.is_empty() {
        return Err(ApiError::bad_request("未配置 API key 时不允许强制覆盖配置"));
    }

    let current = state.config.read().await.clone();
    let mut config = Config {
        api: current.api,
        ..Config::default()
    };
    config.apply_env_overrides(false);
    let mut account = config.accounts().first().cloned().unwrap_or_default();
    if let Some(username) = req.username {
        account.username = username;
    }
    if let Some(password) = req.password {
        account.password = password;
    }
    if !account.username.is_empty() || !account.password.is_empty() {
        config.set_accounts(vec![account]);
    }
    config.save_to_file(&state.config_path)?;
    *state.config.write().await = config.clone();
    *state.sessions.write().await = Vec::new();
    push_log(&state, "info", "已创建/重置配置文件").await;

    Ok(Json(ConfigResponse {
        config_path: state.config_path.clone(),
        exists: true,
        config: PublicConfig::from_config(&config),
    }))
}

async fn list_albums(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DiscListItem>>, ApiError> {
    authorize(&state, &headers).await?;
    let sessions = ensure_logged_in(&state).await?;
    let mut albums_by_id = std::collections::BTreeMap::new();
    for session in sessions {
        for album in session.client.get_my_discs(&session.token).await? {
            albums_by_id.entry(album.id.clone()).or_insert(album);
        }
    }
    let albums = albums_by_id.into_values().collect::<Vec<_>>();
    push_log(
        &state,
        "info",
        format!("已加载 {} 张已购专辑", albums.len()),
    )
    .await;
    Ok(Json(albums))
}

async fn get_album(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DiscInfo>, ApiError> {
    authorize(&state, &headers).await?;
    let sessions = ensure_logged_in(&state).await?;
    let mut last_error = None;
    for session in sessions {
        match session.client.get_disc_info(&id, &session.token).await {
            Ok(album) => return Ok(Json(album)),
            Err(e) => last_error = Some(e),
        }
    }
    Err(last_error
        .map(ApiError::from)
        .unwrap_or_else(|| ApiError::bad_request("未配置 Dizzylab 账号")))
}

async fn start_sync(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Option<Json<SyncRequest>>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    authorize(&state, &headers).await?;
    let album_id = body.and_then(|Json(req)| req.id);
    start_job(state, album_id).await
}

async fn start_album_sync(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    authorize(&state, &headers).await?;
    start_job(state, Some(id)).await
}

async fn start_job(
    state: ApiState,
    album_id: Option<String>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    {
        let mut job = state.job.lock().await;
        if matches!(*job, JobState::Running { .. }) {
            return Err(ApiError::conflict("已有同步任务正在运行"));
        }
        *job = JobState::Running {
            kind: album_id
                .as_ref()
                .map(|id| format!("album:{id}"))
                .unwrap_or_else(|| "all".to_string()),
            started_at: now_unix(),
        };
    }

    *state.last_error.write().await = None;
    push_log(
        &state,
        "info",
        album_id
            .as_ref()
            .map(|id| format!("同步任务已启动：专辑 {id}"))
            .unwrap_or_else(|| "同步任务已启动：全部专辑".to_string()),
    )
    .await;
    let job_state = state.job.clone();
    let last_error = state.last_error.clone();
    let logs = state.logs.clone();

    tokio::spawn(async move {
        let job_handle = tokio::spawn(async move { run_sync_job(state, album_id).await });
        match job_handle.await {
            Ok(Ok(())) => {
                push_log_raw(&logs, "info", "同步任务已完成").await;
            }
            Ok(Err(e)) => {
                error!("API 触发的同步任务失败: {}", e);
                push_log_raw(&logs, "error", format!("同步任务失败：{e}")).await;
                *last_error.write().await = Some(e.to_string());
            }
            Err(e) => {
                error!("API 触发的同步任务异常: {}", e);
                push_log_raw(&logs, "error", format!("同步任务异常：{e}")).await;
                *last_error.write().await = Some(e.to_string());
            }
        }
        *job_state.lock().await = JobState::Idle;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(MessageResponse {
            message: "同步任务已启动".to_string(),
        }),
    ))
}

async fn run_sync_job(state: ApiState, album_id: Option<String>) -> Result<()> {
    let sessions = ensure_logged_in(&state).await?;
    let config = state.config.read().await.clone();
    let mut failures = Vec::new();
    let mut album_found = false;

    for session in sessions {
        let account_label = account_label(&session.account);
        let downloader = Downloader::new(
            session.client.clone(),
            config.clone(),
            session.token.clone(),
        );

        if let Some(album_id) = &album_id {
            match session.client.get_disc_info(album_id, &session.token).await {
                Ok(disc_info) => {
                    album_found = true;
                    info!("账号 {} 开始同步专辑 {}", account_label, album_id);
                    if let Err(e) = downloader.download_album(&disc_info).await {
                        failures.push(format!("{account_label}: {e}"));
                    }
                }
                Err(e) => {
                    info!(
                        "账号 {} 未找到或无法访问专辑 {}: {}",
                        account_label, album_id, e
                    );
                }
            }
        } else {
            match session.client.get_my_discs(&session.token).await {
                Ok(albums) => {
                    info!("账号 {} 开始同步 {} 个专辑", account_label, albums.len());
                    if let Err(e) = downloader.sync_all_albums(albums).await {
                        failures.push(format!("{account_label}: {e}"));
                    }
                }
                Err(e) => failures.push(format!("{account_label}: {e}")),
            }
        }
    }

    if album_id.is_some() && !album_found {
        failures.push("所有账号均未找到或无法访问指定专辑".to_string());
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(failures.join("; ")))
    }
}

async fn ensure_logged_in(state: &ApiState) -> Result<Vec<AccountSession>> {
    let sessions = state.sessions.read().await.clone();
    if !sessions.is_empty() {
        return Ok(sessions);
    }

    let config = state.config.read().await.clone();
    let sessions = login_accounts(&config).await?;
    *state.sessions.write().await = sessions.clone();
    Ok(sessions)
}

async fn login_accounts(config: &Config) -> Result<Vec<AccountSession>> {
    validate_credentials(config)?;
    validate_formats(config)?;

    let mut sessions = Vec::new();
    for account in config.accounts() {
        let client = DizzylabClient::new(config.behavior.debug)?;
        let token = client
            .login(&account.username, &account.password)
            .await
            .map_err(|e| anyhow!("{}: {e}", account_label(&account)))?;
        let user = client.get_my_info(&token).await.ok();
        sessions.push(AccountSession {
            account,
            client,
            token,
            user,
        });
    }
    Ok(sessions)
}

fn account_label(account: &UserConfig) -> String {
    if account.username.trim().is_empty() {
        "<empty>".to_string()
    } else {
        account.username.clone()
    }
}

async fn authorize(state: &ApiState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = state.config.read().await.api.api_key.clone();
    if expected.is_empty() {
        return Ok(());
    }

    let provided = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        });

    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("无效或缺失 API key"))
    }
}

fn apply_config_update(config: &mut Config, req: UpdateConfigRequest) {
    if let Some(users) = req.users {
        let existing = config.accounts();
        let next_users = users
            .into_iter()
            .map(|user| {
                let username = user.username.unwrap_or_default();
                let password = user
                    .password
                    .filter(|password| !password.is_empty())
                    .or_else(|| {
                        existing
                            .iter()
                            .find(|account| account.username == username)
                            .map(|account| account.password.clone())
                    })
                    .unwrap_or_default();

                UserConfig { username, password }
            })
            .collect::<Vec<_>>();
        config.set_accounts(next_users);
    } else if let Some(user) = req.user {
        let mut next = config.accounts().first().cloned().unwrap_or_default();
        if let Some(username) = user.username {
            next.username = username;
        }
        if let Some(password) = user.password {
            if !password.is_empty() {
                next.password = password;
            }
        }
        config.set_accounts(vec![next]);
    }

    if let Some(download) = req.download {
        if let Some(formats) = download.formats {
            config.download.formats = formats;
        }
    }

    if let Some(paths) = req.paths {
        if let Some(output_dir) = paths.output_dir {
            config.paths.output_dir = PathBuf::from(output_dir);
        }
        if let Some(directory_template) = paths.directory_template {
            config.paths.directory_template = directory_template;
        }
    }

    if let Some(behavior) = req.behavior {
        if let Some(skip_existing) = behavior.skip_existing {
            config.behavior.skip_existing = skip_existing;
        }
        if let Some(single_threaded) = behavior.single_threaded {
            config.behavior.single_threaded = single_threaded;
        }
        if let Some(max_concurrent_albums) = behavior.max_concurrent_albums {
            config.behavior.max_concurrent_albums = max_concurrent_albums.max(1);
        }
        if let Some(generate_readme) = behavior.generate_readme {
            config.behavior.generate_readme = generate_readme;
        }
        if let Some(generate_nfo) = behavior.generate_nfo {
            config.behavior.generate_nfo = generate_nfo;
        }
        if let Some(debug) = behavior.debug {
            config.behavior.debug = debug;
        }
        if let Some(metadata_only) = behavior.metadata_only {
            config.behavior.metadata_only = metadata_only;
        }
    }

    if let Some(api) = req.api {
        if let Some(api_key) = api.api_key {
            config.api.api_key = api_key;
        }
    }
}

pub fn validate_credentials(config: &Config) -> Result<()> {
    if !has_credentials(config) {
        return Err(anyhow!("请设置 Dizzylab username 和 password"));
    }
    Ok(())
}

fn has_credentials(config: &Config) -> bool {
    let accounts = config.accounts();
    !accounts.is_empty()
        && accounts.iter().all(|account| {
            !account.username.trim().is_empty() && !account.password.trim().is_empty()
        })
}

pub fn validate_formats(config: &Config) -> Result<()> {
    if config.download.formats.is_empty() {
        return Err(anyhow!("formats 至少需要包含一种下载格式"));
    }

    let mut seen = std::collections::HashSet::new();
    for format in &config.download.formats {
        match format.as_str() {
            "128" | "320" | "FLAC" | "gift" => {}
            _ => {
                return Err(anyhow!(
                    "不支持的下载格式 \"{}\"；可选值为 128、320、FLAC、gift",
                    format
                ));
            }
        }
        if !seen.insert(format.as_str()) {
            return Err(anyhow!("formats 中包含重复的下载格式 \"{}\"", format));
        }
    }

    let has_128 = seen.contains("128");
    let has_320 = seen.contains("320");
    if has_128 && has_320 {
        return Err(anyhow!(
            "formats 中不能同时包含 \"128\" 和 \"320\"：两者均输出 .mp3 文件，文件名会冲突"
        ));
    }
    Ok(())
}

impl PublicConfig {
    fn from_config(config: &Config) -> Self {
        let users = config
            .accounts()
            .into_iter()
            .map(|account| PublicUserConfig {
                username: account.username,
                has_password: !account.password.is_empty(),
            })
            .collect::<Vec<_>>();
        Self {
            user: users.first().cloned().unwrap_or(PublicUserConfig {
                username: String::new(),
                has_password: false,
            }),
            users,
            download: PublicDownloadConfig {
                formats: config.download.formats.clone(),
            },
            paths: PublicPathsConfig {
                output_dir: config.paths.output_dir.display().to_string(),
                directory_template: config.paths.directory_template.clone(),
                output_dir_locked: std::env::var("DIZZYSYNC_OUTPUT_DIR").is_ok(),
            },
            behavior: PublicBehaviorConfig {
                skip_existing: config.behavior.skip_existing,
                single_threaded: config.behavior.single_threaded,
                max_concurrent_albums: config.behavior.max_concurrent_albums.max(1),
                max_concurrent_albums_locked: false,
                generate_readme: config.behavior.generate_readme,
                generate_nfo: config.behavior.generate_nfo,
                debug: config.behavior.debug,
                metadata_only: config.behavior.metadata_only,
            },
            api: PublicApiConfig {
                bind: config.api.bind.clone(),
                has_api_key: !config.api.api_key.is_empty(),
                web_root: config.api.web_root.display().to_string(),
            },
        }
    }
}

fn ensure_api_key_for_remote_bind(config: &mut Config) -> Result<()> {
    if !config.api.api_key.is_empty() || is_loopback_bind(&config.api.bind) {
        return Ok(());
    }

    config.api.api_key = generate_api_key()?;
    info!("API/Web 监听非本地地址且未配置 Web UI 密码，已自动生成并写入配置文件");
    Ok(())
}

fn is_loopback_bind(bind: &str) -> bool {
    if let Ok(socket_addr) = bind.parse::<SocketAddr>() {
        return socket_addr.ip().is_loopback();
    }

    let Some((host, port)) = bind.rsplit_once(':') else {
        return false;
    };

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip.is_loopback();
    }

    format!("{host}:{port}")
        .to_socket_addrs()
        .map(|mut addrs| addrs.all(|addr| addr.ip().is_loopback()))
        .unwrap_or(false)
}

fn generate_api_key() -> Result<String> {
    let mut bytes = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut bytes))
        .map_err(|e| anyhow!("无法从系统随机源读取安全随机数: {e}"))?;

    let mut key = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(key, "{byte:02x}");
    }
    Ok(key)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

async fn push_log(state: &ApiState, level: &'static str, message: impl Into<String>) {
    push_log_raw(&state.logs, level, message).await;
}

async fn push_log_raw(
    logs: &Arc<Mutex<Vec<LogEntry>>>,
    level: &'static str,
    message: impl Into<String>,
) {
    let mut logs = logs.lock().await;
    logs.push(LogEntry {
        timestamp: now_unix(),
        level,
        message: message.into(),
    });
    if logs.len() > 200 {
        let excess = logs.len() - 200;
        logs.drain(0..excess);
    }
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn bad_request(message: impl ToString) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.to_string(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: value.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(MessageResponse {
                message: self.message,
            }),
        )
            .into_response()
    }
}

#[allow(dead_code)]
async fn api_not_found(_request: Request<Body>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(MessageResponse {
            message: "not found".to_string(),
        }),
    )
}
