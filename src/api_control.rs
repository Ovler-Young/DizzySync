use crate::client::DizzylabClient;
use crate::config::Config;
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
    client: Arc<RwLock<Option<DizzylabClient>>>,
    token: Arc<RwLock<Option<String>>>,
    user: Arc<RwLock<Option<UserInfo>>>,
    config: Arc<RwLock<Config>>,
    config_path: String,
    job: Arc<Mutex<JobState>>,
    last_error: Arc<RwLock<Option<String>>>,
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
    user: Option<UserInfo>,
    job: JobState,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
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
    download: PublicDownloadConfig,
    paths: PublicPathsConfig,
    behavior: PublicBehaviorConfig,
    api: PublicApiConfig,
}

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicBehaviorConfig {
    skip_existing: bool,
    single_threaded: bool,
    max_concurrent_albums: usize,
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
    ensure_api_key_for_remote_bind(&mut config);
    config.save_to_file(&options.config_path)?;

    let state = ApiState {
        client: Arc::new(RwLock::new(None)),
        token: Arc::new(RwLock::new(None)),
        user: Arc::new(RwLock::new(None)),
        config: Arc::new(RwLock::new(config.clone())),
        config_path: options.config_path,
        job: Arc::new(Mutex::new(JobState::Idle)),
        last_error: Arc::new(RwLock::new(None)),
    };

    if let Err(e) = ensure_logged_in(&state).await {
        error!("API 服务启动时登录失败: {}", e);
        *state.last_error.write().await = Some(e.to_string());
    }

    let bind = config.api.bind.clone();
    let web_root = config.api.web_root.clone();
    let api = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
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

async fn status(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<StatusResponse>, ApiError> {
    authorize(&state, &headers).await?;
    Ok(Json(StatusResponse {
        status: "ok",
        ready: state.token.read().await.is_some(),
        user: state.user.read().await.clone(),
        job: state.job.lock().await.clone(),
        last_error: state.last_error.read().await.clone(),
    }))
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
    apply_config_update(&mut next_config, req)?;
    validate_credentials(&next_config).map_err(ApiError::bad_request)?;
    validate_formats(&next_config).map_err(ApiError::bad_request)?;

    // Validate the new credentials before committing the config to memory or disk.
    let next_client = DizzylabClient::new(next_config.behavior.debug)?;
    let next_token = next_client
        .login(&next_config.user.username, &next_config.user.password)
        .await
        .map_err(|e| ApiError::unauthorized(format!("登录失败: {e}")))?;
    let next_user = next_client.get_my_info(&next_token).await.ok();

    next_config.save_to_file(&state.config_path)?;
    *state.config.write().await = next_config;
    *state.client.write().await = Some(next_client);
    *state.token.write().await = Some(next_token);
    *state.user.write().await = next_user;

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
    let mut config = Config::default();
    config.api = current.api;
    config.apply_env_overrides(false);
    if let Some(username) = req.username {
        config.user.username = username;
    }
    if let Some(password) = req.password {
        config.user.password = password;
    }
    config.save_to_file(&state.config_path)?;
    *state.config.write().await = config.clone();
    *state.client.write().await = None;
    *state.token.write().await = None;
    *state.user.write().await = None;

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
    let (client, token) = ensure_logged_in(&state).await?;
    let albums = client.get_my_discs(&token).await?;
    Ok(Json(albums))
}

async fn get_album(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DiscInfo>, ApiError> {
    authorize(&state, &headers).await?;
    let (client, token) = ensure_logged_in(&state).await?;
    let album = client.get_disc_info(&id, &token).await?;
    Ok(Json(album))
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
    let job_state = state.job.clone();
    let last_error = state.last_error.clone();

    tokio::spawn(async move {
        let result = run_sync_job(state.clone(), album_id).await;
        if let Err(e) = result {
            error!("API 触发的同步任务失败: {}", e);
            *last_error.write().await = Some(e.to_string());
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
    let (client, token) = ensure_logged_in(&state).await?;
    let config = state.config.read().await.clone();
    let downloader = Downloader::new(client.clone(), config, token.clone());

    if let Some(album_id) = album_id {
        let disc_info = client.get_disc_info(&album_id, &token).await?;
        downloader.download_album(&disc_info).await?;
    } else {
        let albums = client.get_my_discs(&token).await?;
        downloader.sync_all_albums(albums).await?;
    }

    Ok(())
}

async fn ensure_logged_in(state: &ApiState) -> Result<(DizzylabClient, String)> {
    if let (Some(client), Some(token)) = (
        state.client.read().await.clone(),
        state.token.read().await.clone(),
    ) {
        return Ok((client, token));
    }

    let config = state.config.read().await.clone();
    validate_credentials(&config)?;
    validate_formats(&config)?;

    let client = DizzylabClient::new(config.behavior.debug)?;
    let token = client
        .login(&config.user.username, &config.user.password)
        .await
        .map_err(|e| anyhow!("登录失败: {e}"))?;

    let user = client.get_my_info(&token).await.ok();
    *state.client.write().await = Some(client.clone());
    *state.token.write().await = Some(token.clone());
    *state.user.write().await = user;
    Ok((client, token))
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

fn apply_config_update(config: &mut Config, req: UpdateConfigRequest) -> Result<()> {
    if let Some(user) = req.user {
        if let Some(username) = user.username {
            config.user.username = username;
        }
        if let Some(password) = user.password {
            if !password.is_empty() {
                config.user.password = password;
            }
        }
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

    Ok(())
}

pub fn validate_credentials(config: &Config) -> Result<()> {
    if config.user.username.is_empty() || config.user.password.is_empty() {
        return Err(anyhow!("请设置 Dizzylab username 和 password"));
    }
    Ok(())
}

pub fn validate_formats(config: &Config) -> Result<()> {
    let has_128 = config.download.formats.iter().any(|format| format == "128");
    let has_320 = config.download.formats.iter().any(|format| format == "320");
    if has_128 && has_320 {
        return Err(anyhow!(
            "formats 中不能同时包含 \"128\" 和 \"320\"：两者均输出 .mp3 文件，文件名会冲突"
        ));
    }
    Ok(())
}

impl PublicConfig {
    fn from_config(config: &Config) -> Self {
        Self {
            user: PublicUserConfig {
                username: config.user.username.clone(),
                has_password: !config.user.password.is_empty(),
            },
            download: PublicDownloadConfig {
                formats: config.download.formats.clone(),
            },
            paths: PublicPathsConfig {
                output_dir: config.paths.output_dir.display().to_string(),
                directory_template: config.paths.directory_template.clone(),
            },
            behavior: PublicBehaviorConfig {
                skip_existing: config.behavior.skip_existing,
                single_threaded: config.behavior.single_threaded,
                max_concurrent_albums: config.behavior.max_concurrent_albums,
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

fn ensure_api_key_for_remote_bind(config: &mut Config) {
    if !config.api.api_key.is_empty() || is_loopback_bind(&config.api.bind) {
        return;
    }

    config.api.api_key = generate_api_key();
    info!("API/Web 监听非本地地址且未配置 API key，已自动生成并写入配置文件");
}

fn is_loopback_bind(bind: &str) -> bool {
    bind.starts_with("127.") || bind.starts_with("localhost:") || bind.starts_with("[::1]:")
}

fn generate_api_key() -> String {
    let mut bytes = [0_u8; 32];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut bytes))
        .is_err()
    {
        let now = now_unix().to_le_bytes();
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = now[index % now.len()] ^ (index as u8).wrapping_mul(31);
        }
    }

    let mut key = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(key, "{byte:02x}");
    }
    key
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
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
