use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub user: UserConfig,
    pub download: DownloadConfig,
    pub paths: PathsConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub formats: Vec<String>, // "128", "MP3", "FLAC", "gift"
    pub flatten: bool,
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
    #[serde(default = "default_true")]
    pub generate_readme: bool,
    #[serde(default = "default_true")]
    pub generate_nfo: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user: UserConfig {
                cookie: String::new(),
            },
            download: DownloadConfig {
                formats: vec!["MP3".to_string(), "FLAC".to_string()],
                flatten: false,
            },
            paths: PathsConfig {
                output_dir: PathBuf::from("./DizzySync"),
                directory_template: "{album}/@{label}".to_string(),
            },
            behavior: BehaviorConfig {
                skip_existing: true,
                single_threaded: true,
                generate_readme: true,
                generate_nfo: true,
            },
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
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn create_default_config(path: &str) -> Result<()> {
        let default_config = Config::default();
        default_config.save_to_file(path)?;
        println!("已创建默认配置文件: {}", path);
        println!("请编辑配置文件，设置你的cookie等信息");
        Ok(())
    }
} 