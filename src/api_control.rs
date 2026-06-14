use crate::client::DizzylabClient;
use crate::config::{Config, UserConfig};
use crate::downloader::Downloader;
use crate::local_state;
use crate::types::{DiscInfo, DiscListItem, UserInfo};
use anyhow::{anyhow, Result};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::{Path as StdPath, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tokio_util::io::ReaderStream;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info, Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

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
    job: Arc<TokioMutex<JobState>>,
    schedule: Arc<RwLock<ScheduleState>>,
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

#[derive(Debug, Clone, Serialize)]
struct ScheduleState {
    enabled: bool,
    cron: String,
    next_run: Option<u64>,
    last_run: Option<u64>,
    last_error: Option<String>,
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
    schedule: ScheduleState,
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

#[derive(Debug, Deserialize)]
struct LogQuery {
    date: Option<String>,
    level: Option<String>,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFileQuery {
    path: String,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlbumsQuery {
    #[serde(default)]
    refresh: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AlbumCacheFile {
    cache_key: String,
    updated_at: u64,
    albums: Vec<DiscListItem>,
}

static WEB_LOGS: OnceLock<Arc<Mutex<Vec<LogEntry>>>> = OnceLock::new();
const MAX_LOG_ENTRIES: usize = 1000;

pub fn web_log_layer() -> WebLogLayer {
    WebLogLayer
}

pub struct WebLogLayer;

impl<S> Layer<S> for WebLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = match *event.metadata().level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };
        let mut visitor = LogMessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.finish();
        if !message.is_empty() {
            push_log_sync(shared_logs(), level, message);
        }
    }
}

#[derive(Default)]
struct LogMessageVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl LogMessageVisitor {
    fn finish(self) -> String {
        match (self.message, self.fields.is_empty()) {
            (Some(message), true) => message,
            (Some(message), false) => format!("{} {}", message, self.fields.join(" ")),
            (None, false) => self.fields.join(" "),
            (None, true) => String::new(),
        }
    }
}

impl tracing::field::Visit for LogMessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}").trim_matches('"').to_string());
        } else {
            self.fields.push(format!("{}={value:?}", field.name()));
        }
    }
}

fn shared_logs() -> &'static Arc<Mutex<Vec<LogEntry>>> {
    WEB_LOGS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
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
    schedule: PublicScheduleConfig,
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
struct PublicScheduleConfig {
    enabled: bool,
    cron: String,
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
    schedule: Option<UpdateScheduleConfig>,
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
struct UpdateScheduleConfig {
    enabled: Option<bool>,
    cron: Option<String>,
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
struct TestLoginRequest {
    username: String,
    password: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestLoginResponse {
    success: bool,
    account_username: String,
    user: Option<UserInfo>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct SyncRequest {
    id: Option<String>,
}

pub async fn run(options: ApiServerOptions) -> Result<()> {
    let mut config = options.config.clone();
    validate_schedule(&config)?;
    ensure_api_key_for_remote_bind(&mut config)?;
    config.save_to_file(&options.config_path)?;

    let state = ApiState {
        sessions: Arc::new(RwLock::new(Vec::new())),
        config: Arc::new(RwLock::new(config.clone())),
        config_path: options.config_path,
        job: Arc::new(TokioMutex::new(JobState::Idle)),
        schedule: Arc::new(RwLock::new(schedule_state_from_config(&config))),
        last_error: Arc::new(RwLock::new(None)),
        logs: shared_logs().clone(),
    };
    push_log(&state, "info", "API/Web 控制服务初始化完成").await;

    if let Err(e) = ensure_logged_in(&state).await {
        error!("API 服务启动时登录失败: {}", e);
        *state.last_error.write().await = Some(e.to_string());
    }

    start_scheduler(state.clone());

    let bind = config.api.bind.clone();
    let web_root = config.api.web_root.clone();
    let api = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/logs", get(get_logs))
        .route("/config", get(get_config).put(update_config))
        .route("/config/bootstrap", post(bootstrap_config))
        .route("/config/test-login", post(test_login))
        .route("/albums", get(list_albums))
        .route("/albums/{id}", get(get_album))
        .route("/local-file", get(get_local_file))
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

async fn status(State(state): State<ApiState>, headers: HeaderMap) -> Json<StatusResponse> {
    let config = state.config.read().await.clone();
    let authenticated = authorize_with_key(&config.api.api_key, header_api_key(&headers)).is_ok();
    let sessions = state.sessions.read().await.clone();
    let users = sessions
        .iter()
        .filter_map(|session| session.user.clone())
        .collect::<Vec<_>>();
    let schedule = {
        let schedule = state.schedule.read().await;
        if authenticated {
            schedule.clone()
        } else {
            redacted_schedule_state(&schedule)
        }
    };

    Json(StatusResponse {
        status: "ok",
        ready: !sessions.is_empty(),
        configured: has_credentials(&config),
        requires_auth: !config.api.api_key.is_empty(),
        user: authenticated.then(|| users.first().cloned()).flatten(),
        users: if authenticated { users } else { Vec::new() },
        job: if authenticated {
            state.job.lock().await.clone()
        } else {
            JobState::Idle
        },
        schedule,
        last_error: if authenticated {
            state.last_error.read().await.clone()
        } else {
            None
        },
    })
}

async fn get_logs(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<LogEntry>>, ApiError> {
    authorize(&state, &headers).await?;
    let logs = state
        .logs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(Json(filter_logs(&logs, &query)))
}

fn filter_logs(logs: &[LogEntry], query: &LogQuery) -> Vec<LogEntry> {
    let level = query.level.as_deref().filter(|level| !level.is_empty());
    let start = query.start.as_deref().and_then(parse_log_time);
    let end = query.end.as_deref().and_then(parse_log_time);

    logs.iter()
        .filter(|entry| match level {
            Some(level) => entry.level.eq_ignore_ascii_case(level),
            None => true,
        })
        .filter(|entry| match query.date.as_deref() {
            Some(date) => log_date(entry.timestamp) == date,
            None => true,
        })
        .filter(|entry| match start {
            Some(start) => entry.timestamp >= start,
            None => true,
        })
        .filter(|entry| match end {
            Some(end) => entry.timestamp <= end,
            None => true,
        })
        .cloned()
        .collect()
}

fn parse_log_time(value: &str) -> Option<u64> {
    if let Ok(timestamp) = value.parse::<u64>() {
        return Some(timestamp);
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.timestamp().max(0) as u64)
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
                .ok()
                .and_then(|datetime| datetime.and_local_timezone(chrono::Local).single())
                .map(|datetime| datetime.timestamp().max(0) as u64)
        })
}

fn log_date(timestamp: u64) -> String {
    chrono::DateTime::from_timestamp(timestamp as i64, 0)
        .map(|datetime| {
            datetime
                .with_timezone(&chrono::Local)
                .date_naive()
                .to_string()
        })
        .unwrap_or_default()
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
    validate_schedule(&next_config).map_err(ApiError::bad_request)?;

    // Validate all credentials before committing the config to memory or disk.
    let next_sessions = login_accounts(&next_config)
        .await
        .map_err(|e| ApiError::unauthorized(format!("登录失败: {e}")))?;

    next_config.save_to_file(&state.config_path)?;
    *state.config.write().await = next_config.clone();
    *state.sessions.write().await = next_sessions;
    *state.schedule.write().await = schedule_state_from_config(&next_config);
    push_log(&state, "info", "配置已验证并保存").await;

    let config = state.config.read().await.clone();
    Ok(Json(ConfigResponse {
        config_path: state.config_path.clone(),
        exists: std::path::Path::new(&state.config_path).exists(),
        config: PublicConfig::from_config(&config),
    }))
}

async fn test_login(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(req): Json<TestLoginRequest>,
) -> Result<Json<TestLoginResponse>, ApiError> {
    authorize(&state, &headers).await?;

    let username = req.username.trim().to_string();
    if username.is_empty() {
        return Err(ApiError::bad_request("请设置 Dizzylab username"));
    }

    let config = state.config.read().await.clone();
    let password = req
        .password
        .as_deref()
        .map(str::trim)
        .filter(|password| !password.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            config
                .accounts()
                .into_iter()
                .find(|account| account.username == username)
                .map(|account| account.password)
        })
        .unwrap_or_default();

    if password.trim().is_empty() {
        return Err(ApiError::bad_request("请设置 Dizzylab password"));
    }

    let account = UserConfig {
        username: username.clone(),
        password,
    };
    let account_label = account_label(&account);
    let client = DizzylabClient::new(config.behavior.debug)?;

    match client.login(&account.username, &account.password).await {
        Ok(token) => match client.get_my_info(&token).await {
            Ok(user) => {
                let message = format!(
                    "账号 {account_label} 登录成功，已获取用户 {} (UID: {})",
                    user.username, user.uid
                );
                push_log(&state, "info", message.clone()).await;
                Ok(Json(TestLoginResponse {
                    success: true,
                    account_username: account.username,
                    user: Some(user),
                    message,
                }))
            }
            Err(e) => {
                let message = format!("账号 {account_label} 登录成功，但获取用户信息失败: {e}");
                push_log(&state, "warn", message.clone()).await;
                Ok(Json(TestLoginResponse {
                    success: true,
                    account_username: account.username,
                    user: None,
                    message,
                }))
            }
        },
        Err(e) => {
            let message = format!("账号 {account_label} 登录失败: {e}");
            push_log(&state, "warn", message.clone()).await;
            Ok(Json(TestLoginResponse {
                success: false,
                account_username: account.username,
                user: None,
                message,
            }))
        }
    }
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
    *state.schedule.write().await = schedule_state_from_config(&config);
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
    Query(query): Query<AlbumsQuery>,
) -> Result<Json<Vec<DiscListItem>>, ApiError> {
    authorize(&state, &headers).await?;
    let config = state.config.read().await.clone();
    let sessions = ensure_logged_in(&state).await?;
    let cache_key = album_cache_key(&sessions);

    if !query.refresh {
        if let Some(mut albums) = read_album_cache(&state.config_path, &cache_key).await {
            local_state::annotate_album_list(&config, &mut albums);
            push_log(
                &state,
                "debug",
                format!("已从本地缓存加载 {} 张已购专辑", albums.len()),
            )
            .await;
            return Ok(Json(albums));
        }
    }

    let mut albums_by_id = std::collections::BTreeMap::new();
    for session in &sessions {
        for album in session.client.get_my_discs(&session.token).await? {
            albums_by_id.entry(album.id.clone()).or_insert(album);
        }
    }
    let albums = albums_by_id.into_values().collect::<Vec<_>>();
    write_album_cache(&state.config_path, &cache_key, &albums).await;

    let mut annotated = albums;
    local_state::annotate_album_list(&config, &mut annotated);
    push_log(
        &state,
        "info",
        format!("已加载 {} 张已购专辑", annotated.len()),
    )
    .await;
    Ok(Json(annotated))
}

async fn get_album(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DiscInfo>, ApiError> {
    authorize(&state, &headers).await?;
    if id.trim().is_empty() {
        return Err(ApiError::bad_request("专辑 ID 不能为空"));
    }
    let sessions = ensure_logged_in(&state).await?;
    let mut last_error = None;
    for session in &sessions {
        match session.client.get_disc_info(&id, &session.token).await {
            Ok(mut album) => {
                let config = state.config.read().await.clone();
                local_state::annotate_disc_info(&config, &mut album);
                return Ok(Json(album));
            }
            Err(e) => last_error = Some(e),
        }
    }
    if sessions.is_empty() {
        return Err(ApiError::bad_request("未配置 Dizzylab 账号"));
    }
    let message = last_error
        .map(|e| format!("未找到或无法访问专辑 {id}: {e}"))
        .unwrap_or_else(|| format!("未找到或无法访问专辑 {id}"));
    Err(ApiError::not_found(message))
}

async fn get_local_file(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<LocalFileQuery>,
) -> Result<Response, ApiError> {
    let config = state.config.read().await.clone();
    authorize_with_key(
        &config.api.api_key,
        query
            .api_key
            .as_deref()
            .or_else(|| header_api_key(&headers)),
    )?;

    let requested = PathBuf::from(&query.path);
    let canonical_file = requested
        .canonicalize()
        .map_err(|_| ApiError::not_found("本地文件不存在"))?;
    let canonical_output = config
        .paths
        .output_dir
        .canonicalize()
        .map_err(|_| ApiError::not_found("输出目录不存在"))?;

    if !canonical_file.starts_with(&canonical_output) || !canonical_file.is_file() {
        return Err(ApiError::not_found("本地文件不存在"));
    }
    if !is_supported_audio_file(&canonical_file) {
        return Err(ApiError::bad_request("不支持的本地文件类型"));
    }

    let content_type = content_type_for_audio(&canonical_file);
    let file_name = canonical_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("audio");

    let file = tokio::fs::File::open(&canonical_file)
        .await
        .map_err(|e| ApiError::internal(format!("读取本地文件失败: {e}")))?;
    let file_len = file.metadata().await.ok().map(|metadata| metadata.len());
    let stream = ReaderStream::new(file);
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        content_type
            .parse()
            .unwrap_or(header::HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("inline; filename=\"{}\"", sanitize_header_value(file_name))
            .parse()
            .unwrap_or(header::HeaderValue::from_static("inline")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );
    if let Some(file_len) = file_len {
        if let Ok(value) = file_len.to_string().parse() {
            response.headers_mut().insert(header::CONTENT_LENGTH, value);
        }
    }
    Ok(response)
}

fn is_supported_audio_file(path: &StdPath) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "wav" | "m4a" | "ogg"
            )
        })
        .unwrap_or(false)
}

fn content_type_for_audio(path: &StdPath) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    }
}

fn sanitize_header_value(value: &str) -> String {
    value.replace(['\r', '\n', '"'], "_")
}

fn redacted_schedule_state(schedule: &ScheduleState) -> ScheduleState {
    ScheduleState {
        enabled: schedule.enabled,
        cron: if schedule.enabled {
            "<redacted>".to_string()
        } else {
            String::new()
        },
        next_run: schedule.next_run,
        last_run: schedule.last_run,
        last_error: None,
    }
}

fn album_cache_key(sessions: &[AccountSession]) -> String {
    let mut identities = sessions
        .iter()
        .map(|session| {
            let uid = session
                .user
                .as_ref()
                .map(|user| user.uid.as_str())
                .unwrap_or("");
            format!("{}:{uid}", session.account.username)
        })
        .collect::<Vec<_>>();
    identities.sort();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    identities.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn album_cache_path(config_path: &str, cache_key: &str) -> PathBuf {
    let config_path = StdPath::new(config_path);
    let base_dir = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| StdPath::new("."));
    base_dir
        .join(".dizzysync-cache")
        .join(format!("albums-{cache_key}.json"))
}

async fn read_album_cache(config_path: &str, cache_key: &str) -> Option<Vec<DiscListItem>> {
    let cache_path = album_cache_path(config_path, cache_key);
    let content = tokio::fs::read_to_string(cache_path).await.ok()?;
    let cache: AlbumCacheFile = serde_json::from_str(&content).ok()?;
    (cache.cache_key == cache_key).then_some(cache.albums)
}

async fn write_album_cache(config_path: &str, cache_key: &str, albums: &[DiscListItem]) {
    let cache_path = album_cache_path(config_path, cache_key);
    if let Some(parent) = cache_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            tracing::debug!("无法创建专辑缓存目录: {}", e);
            return;
        }
    }
    let cache = AlbumCacheFile {
        cache_key: cache_key.to_string(),
        updated_at: now_unix(),
        albums: albums.to_vec(),
    };
    match serde_json::to_vec_pretty(&cache) {
        Ok(bytes) => {
            if let Err(e) = tokio::fs::write(cache_path, bytes).await {
                tracing::debug!("无法写入专辑缓存: {}", e);
            }
        }
        Err(e) => tracing::debug!("无法序列化专辑缓存: {}", e),
    }
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
    authorize_with_key(&expected, header_api_key(headers))
}

fn header_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        })
}

fn authorize_with_key(expected: &str, provided: Option<&str>) -> Result<(), ApiError> {
    if expected.is_empty() || provided == Some(expected) {
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

    if let Some(schedule) = req.schedule {
        if let Some(enabled) = schedule.enabled {
            config.schedule.enabled = enabled;
        }
        if let Some(cron) = schedule.cron {
            config.schedule.cron = cron.trim().to_string();
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

pub fn validate_schedule(config: &Config) -> Result<()> {
    if !config.schedule.enabled {
        return Ok(());
    }

    let cron = config.schedule.cron.trim();
    if cron.is_empty() {
        return Err(anyhow!("启用自动同步时必须设置 cron 表达式"));
    }
    Schedule::from_str(cron).map_err(|e| anyhow!("无效的 cron 表达式: {e}"))?;
    Ok(())
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
            schedule: PublicScheduleConfig {
                enabled: config.schedule.enabled,
                cron: config.schedule.cron.clone(),
            },
            api: PublicApiConfig {
                bind: config.api.bind.clone(),
                has_api_key: !config.api.api_key.is_empty(),
                web_root: config.api.web_root.display().to_string(),
            },
        }
    }
}

fn start_scheduler(state: ApiState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let config = state.config.read().await.clone();
            if !config.schedule.enabled {
                continue;
            }

            let schedule = match Schedule::from_str(config.schedule.cron.trim()) {
                Ok(schedule) => schedule,
                Err(e) => {
                    let message = format!("无效的 cron 表达式: {e}");
                    error!("{}", message);
                    state.schedule.write().await.last_error = Some(message);
                    continue;
                }
            };

            let now = Utc::now();
            let now_ts = now.timestamp() as u64;
            let next_run_ts = {
                let mut schedule_state = state.schedule.write().await;
                schedule_state.enabled = config.schedule.enabled;
                schedule_state.cron = config.schedule.cron.clone();
                match schedule_state.next_run {
                    Some(next_run) => next_run,
                    None => {
                        let Some(next_run) = schedule.upcoming(Utc).next() else {
                            continue;
                        };
                        let next_run = next_run.timestamp() as u64;
                        schedule_state.next_run = Some(next_run);
                        next_run
                    }
                }
            };

            if now_ts < next_run_ts {
                continue;
            }

            let next_after_fire = schedule
                .upcoming(Utc)
                .find(|datetime| datetime.timestamp() as u64 > now_ts)
                .map(|datetime| datetime.timestamp() as u64);

            {
                let mut schedule_state = state.schedule.write().await;
                schedule_state.next_run = next_after_fire;
            }

            let mut job = state.job.lock().await;
            if matches!(*job, JobState::Running { .. }) {
                info!("自动同步已到触发时间，但已有同步任务正在运行，跳过本次触发");
                continue;
            }
            *job = JobState::Running {
                kind: "scheduled".to_string(),
                started_at: now_unix(),
            };
            drop(job);

            let run_state = state.clone();
            let job_state = state.job.clone();
            let schedule_state = state.schedule.clone();
            let last_error = state.last_error.clone();
            tokio::spawn(async move {
                let started_at = now_unix();
                let current = schedule_state.read().await.clone();
                *schedule_state.write().await = ScheduleState {
                    last_run: Some(started_at),
                    ..current
                };

                let job_handle = tokio::spawn(async move { run_sync_job(run_state, None).await });
                match job_handle.await {
                    Ok(Ok(())) => {
                        let current = schedule_state.read().await.clone();
                        *schedule_state.write().await = ScheduleState {
                            last_error: None,
                            ..current
                        };
                    }
                    Ok(Err(e)) => {
                        let message = e.to_string();
                        error!("自动同步任务失败: {}", message);
                        *last_error.write().await = Some(message.clone());
                        let current = schedule_state.read().await.clone();
                        *schedule_state.write().await = ScheduleState {
                            last_error: Some(message),
                            ..current
                        };
                    }
                    Err(e) => {
                        let message = format!("自动同步任务异常: {e}");
                        error!("{}", message);
                        *last_error.write().await = Some(message.clone());
                        let current = schedule_state.read().await.clone();
                        *schedule_state.write().await = ScheduleState {
                            last_error: Some(message),
                            ..current
                        };
                    }
                }
                *job_state.lock().await = JobState::Idle;
            });
        }
    });
}

fn schedule_state_from_config(config: &Config) -> ScheduleState {
    let next_run = if config.schedule.enabled {
        Schedule::from_str(config.schedule.cron.trim())
            .ok()
            .and_then(|schedule| schedule.upcoming(Utc).next())
            .map(|datetime| datetime.timestamp() as u64)
    } else {
        None
    };

    ScheduleState {
        enabled: config.schedule.enabled,
        cron: config.schedule.cron.clone(),
        next_run,
        last_run: None,
        last_error: None,
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
    push_log_sync(logs, level, message);
}

fn redact_sensitive(message: &str) -> String {
    let mut redacted = message.to_string();
    for pattern in [
        r"(?i)(token=)[^\s&]+",
        r"(?i)(api[_-]?key=)[^\s&]+",
        r"(?i)(password=)[^\s&]+",
        r#"(?i)("token"\s*:\s*)"[^"]*""#,
        r#"(?i)("api[_-]?key"\s*:\s*)"[^"]*""#,
        r#"(?i)("password"\s*:\s*)"[^"]*""#,
        r#"(?i)(token['"]?\s*[:=]\s*)[^,}}\]\s]+"#,
        r#"(?i)(api[_-]?key['"]?\s*[:=]\s*)[^,}}\]\s]+"#,
        r#"(?i)(password['"]?\s*[:=]\s*)[^,}}\]\s]+"#,
    ] {
        if let Ok(regex) = regex::Regex::new(pattern) {
            redacted = regex.replace_all(&redacted, "$1<redacted>").into_owned();
        }
    }
    redacted
}

fn push_log_sync(
    logs: &Arc<Mutex<Vec<LogEntry>>>,
    level: &'static str,
    message: impl Into<String>,
) {
    let mut logs = logs.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    logs.push(LogEntry {
        timestamp: now_unix(),
        level,
        message: redact_sensitive(&message.into()),
    });
    if logs.len() > MAX_LOG_ENTRIES {
        let excess = logs.len() - MAX_LOG_ENTRIES;
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
            message: redact_sensitive(&message.into()),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: redact_sensitive(&message.into()),
        }
    }

    fn bad_request(message: impl ToString) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: redact_sensitive(&message.to_string()),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: redact_sensitive(&message.into()),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: redact_sensitive(&message.into()),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: redact_sensitive(&value.to_string()),
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
