use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub user: UserConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<UserConfig>,
    pub download: DownloadConfig,
    pub paths: PathsConfig,
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub formats: Vec<String>, // "128", "320", "FLAC", "gift"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub output_dir: PathBuf,
    pub directory_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub skip_existing: bool,
    pub single_threaded: bool,
    #[serde(default = "default_one")]
    pub max_concurrent_albums: usize,
    #[serde(default = "default_true")]
    pub generate_readme: bool,
    #[serde(default = "default_true")]
    pub generate_nfo: bool,
    #[serde(default = "default_false")]
    pub debug: bool,
    #[serde(default = "default_false")]
    pub metadata_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_bind")]
    pub bind: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_web_root")]
    pub web_root: PathBuf,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind: default_api_bind(),
            api_key: String::new(),
            web_root: default_web_root(),
        }
    }
}

fn default_one() -> usize {
    1
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_api_bind() -> String {
    "127.0.0.1:8787".to_string()
}

fn default_web_root() -> PathBuf {
    PathBuf::from("./web/dist")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user: UserConfig {
                username: String::new(),
                password: String::new(),
            },
            users: Vec::new(),
            download: DownloadConfig {
                formats: vec!["320".to_string(), "FLAC".to_string()],
            },
            paths: PathsConfig {
                output_dir: PathBuf::from("./DizzySync"),
                directory_template: "{album}/@{label}".to_string(),
            },
            behavior: BehaviorConfig {
                skip_existing: true,
                single_threaded: true,
                max_concurrent_albums: 1,
                generate_readme: true,
                generate_nfo: true,
                debug: false,
                metadata_only: false,
            },
            api: ApiConfig::default(),
        }
    }
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn create_default_config(path: &str) -> Result<()> {
        let default_config = Config::default();
        default_config.save_to_file(path)?;
        println!("已创建默认配置文件: {path}");
        println!("请编辑配置文件，设置你的用户名和密码");
        Ok(())
    }

    pub fn apply_env_overrides(&mut self, fill_empty_only: bool) {
        apply_string_env(
            &mut self.api.api_key,
            "DIZZYSYNC_WEB_PASSWORD",
            fill_empty_only,
        );
        apply_string_env(&mut self.api.api_key, "DIZZYSYNC_API_KEY", fill_empty_only);

        apply_string_env(
            &mut self.user.username,
            "DIZZYSYNC_USERNAME",
            fill_empty_only,
        );
        apply_string_env(
            &mut self.user.password,
            "DIZZYSYNC_PASSWORD",
            fill_empty_only,
        );

        if let Ok(output_dir) = std::env::var("DIZZYSYNC_OUTPUT_DIR") {
            // The deployment-provided output directory is authoritative so the Web UI
            // cannot drift away from mounted storage such as Docker's /data volume.
            self.paths.output_dir = PathBuf::from(output_dir);
        }

        self.behavior.max_concurrent_albums = self.behavior.max_concurrent_albums.max(1);
    }

    pub fn accounts(&self) -> Vec<UserConfig> {
        if self.users.is_empty() {
            if self.user.username.trim().is_empty() && self.user.password.trim().is_empty() {
                Vec::new()
            } else {
                vec![self.user.clone()]
            }
        } else {
            self.users.clone()
        }
    }

    pub fn set_accounts(&mut self, users: Vec<UserConfig>) {
        self.users = users;
        self.user = self.users.first().cloned().unwrap_or_default();
    }

    pub fn load_or_bootstrap(path: &str) -> Result<Self> {
        if std::path::Path::new(path).exists() {
            let mut config = Self::load_from_file(path)?;
            config.apply_env_overrides(true);
            return Ok(config);
        }

        let mut config = Self::default();
        config.apply_env_overrides(false);
        if std::env::var("DIZZYSYNC_OUTPUT_DIR").is_err() {
            config.paths.output_dir = PathBuf::from("/data");
        }
        config.api.web_root = PathBuf::from("/app/web");
        config.save_to_file(path)?;
        Ok(config)
    }
}

fn apply_string_env(value: &mut String, key: &str, fill_empty_only: bool) {
    if let Ok(env_value) = std::env::var(key) {
        if !fill_empty_only || value.is_empty() {
            *value = env_value;
        }
    }
}
