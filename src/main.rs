mod api_control;
mod archive;
mod client;
mod config;
mod downloader;
mod metadata;
mod types;

use anyhow::Result;
use clap::{Arg, Command};
use client::DizzylabClient;
use config::Config;
use downloader::Downloader;
use std::path::Path;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("DizzySync")
        .version("0.1.0")
        .author("去离子水")
        .about("Dizzylab自动同步器")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("配置文件路径")
                .default_value("config.toml"),
        )
        .arg(
            Arg::new("init")
                .long("init")
                .help("创建默认配置文件")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("仅列出专辑，不下载")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("启用调试模式，打印所有HTTP响应")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("metadata-only")
                .long("metadata-only")
                .help("仅下载元数据（专辑信息、封面、README、NFO），不下载音频文件")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .value_name("ALBUM_ID")
                .help("仅下载指定ID的专辑（例如：dts）")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("skip-existing")
                .long("skip-existing")
                .value_name("[BOOL]")
                .help("跳过已存在的目录 [默认: true]")
                .num_args(0..=1)
                .default_missing_value("true")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("single-threaded")
                .long("single-threaded")
                .value_name("[BOOL]")
                .help("单线程模式 [默认: true]")
                .num_args(0..=1)
                .default_missing_value("true")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("generate-readme")
                .long("generate-readme")
                .value_name("[BOOL]")
                .help("生成README.md文件 [默认: true]")
                .num_args(0..=1)
                .default_missing_value("true")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("generate-nfo")
                .long("generate-nfo")
                .value_name("[BOOL]")
                .help("生成NFO文件 [默认: true]")
                .num_args(0..=1)
                .default_missing_value("true")
                .value_parser(clap::value_parser!(bool)),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .short('o')
                .value_name("DIR")
                .help("指定输出目录")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("api-server")
                .long("api-server")
                .help("启动HTTP API与Web控制服务")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("api-bind")
                .long("api-bind")
                .value_name("ADDR")
                .help("API/Web服务监听地址，例如 0.0.0.0:8787")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("api-key")
                .long("api-key")
                .value_name("KEY")
                .help("API访问密钥；设置后请求需携带 X-API-Key 或 Bearer Token")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("web-root")
                .long("web-root")
                .value_name("DIR")
                .help("静态Web前端目录")
                .value_parser(clap::value_parser!(String)),
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();

    let env_filter = if matches.get_flag("debug") {
        EnvFilter::new("dizzysync=debug,scraper=warn,info,html5ever=warn,info,selectors=warn,info")
    } else {
        EnvFilter::new("info")
    };

    fmt().with_env_filter(env_filter).with_target(false).init();

    if matches.get_flag("debug") {
        info!("调试模式已启用，将显示所有HTTP响应");
    }

    if matches.get_flag("init") {
        Config::create_default_config(config_path)?;
        return Ok(());
    }

    let is_api_server = matches.get_flag("api-server");

    if !Path::new(config_path).exists() && !is_api_server {
        error!("配置文件不存在: {}", config_path);
        error!("请运行 'dizzysync --init' 创建默认配置文件");
        return Ok(());
    }

    let mut config = if is_api_server {
        Config::load_or_bootstrap(config_path)?
    } else {
        Config::load_from_file(config_path)?
    };

    if matches.get_flag("debug") {
        config.behavior.debug = true;
    }

    if matches.get_flag("metadata-only") {
        config.behavior.metadata_only = true;
        info!("启用仅元数据模式：只下载专辑信息，不下载音频文件");
    }

    if let Some(skip_existing) = matches.get_one::<bool>("skip-existing") {
        config.behavior.skip_existing = *skip_existing;
        info!("设置跳过已存在目录: {}", skip_existing);
    }

    if let Some(single_threaded) = matches.get_one::<bool>("single-threaded") {
        config.behavior.single_threaded = *single_threaded;
        info!("设置单线程模式: {}", single_threaded);
    }

    if let Some(generate_readme) = matches.get_one::<bool>("generate-readme") {
        config.behavior.generate_readme = *generate_readme;
        info!("设置README.md生成: {}", generate_readme);
    }

    if let Some(generate_nfo) = matches.get_one::<bool>("generate-nfo") {
        config.behavior.generate_nfo = *generate_nfo;
        info!("设置NFO文件生成: {}", generate_nfo);
    }

    if let Some(output_dir) = matches.get_one::<String>("output-dir") {
        if is_api_server && std::env::var("DIZZYSYNC_OUTPUT_DIR").is_ok() {
            info!("已设置 DIZZYSYNC_OUTPUT_DIR，忽略 --output-dir 参数");
        } else {
            config.paths.output_dir = PathBuf::from(output_dir);
            info!("设置输出目录: {}", output_dir);
        }
    }

    if let Some(api_bind) = matches.get_one::<String>("api-bind") {
        config.api.bind = api_bind.clone();
        info!("设置API/Web监听地址: {}", api_bind);
    }

    if let Some(api_key) = matches.get_one::<String>("api-key") {
        config.api.api_key = api_key.clone();
        info!("已设置API访问密钥");
    }

    if let Some(web_root) = matches.get_one::<String>("web-root") {
        config.api.web_root = PathBuf::from(web_root);
        info!("设置Web前端目录: {}", web_root);
    }

    if is_api_server {
        config.apply_env_overrides(true);
        config.save_to_file(config_path)?;
        return api_control::run(api_control::ApiServerOptions {
            config_path: config_path.clone(),
            config,
        })
        .await;
    }

    if let Err(e) = api_control::validate_credentials(&config) {
        error!("{}", e);
        return Ok(());
    }

    if let Err(e) = api_control::validate_formats(&config) {
        error!("{}", e);
        error!("请在配置文件中只保留其中一个");
        return Ok(());
    }

    let accounts = config.accounts();
    let dry_run = matches.get_flag("dry-run");
    let requested_album_id = matches.get_one::<String>("id").cloned();
    let mut failures = Vec::new();
    let mut requested_album_found = false;

    for account in accounts {
        let account_label = if account.username.trim().is_empty() {
            "<empty>".to_string()
        } else {
            account.username.clone()
        };
        info!("账号 {} 登录中", account_label);
        let client = DizzylabClient::new(config.behavior.debug)?;
        let token = match client.login(&account.username, &account.password).await {
            Ok(t) => t,
            Err(e) => {
                error!("账号 {} 登录失败: {}", account_label, e);
                failures.push(format!("{account_label}: {e}"));
                continue;
            }
        };

        if let Ok(user_info) = client.get_my_info(&token).await {
            info!(
                "账号 {} 已登录为: {} (UID: {})",
                account_label, user_info.username, user_info.uid
            );
        }

        let downloader = Downloader::new(client.clone(), config.clone(), token.clone());
        if let Some(album_id) = &requested_album_id {
            info!("账号 {} 获取指定专辑: {}", account_label, album_id);
            match client.get_disc_info(album_id, &token).await {
                Ok(disc_info) => {
                    requested_album_found = true;
                    if dry_run {
                        println!(
                            "[{}] 1. {} - {} ({})",
                            account_label, disc_info.title, disc_info.label, disc_info.id
                        );
                    } else if let Err(e) = downloader.download_album(&disc_info).await {
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
            continue;
        }

        let albums = match client.get_my_discs(&token).await {
            Ok(albums) => albums,
            Err(e) => {
                failures.push(format!("{account_label}: {e}"));
                continue;
            }
        };

        if albums.is_empty() {
            info!("账号 {} 没有找到任何专辑", account_label);
            continue;
        }

        info!("账号 {} 找到 {} 个专辑", account_label, albums.len());

        if dry_run {
            info!("=== 账号 {} 专辑列表 ===", account_label);
            for (index, album) in albums.iter().enumerate() {
                println!(
                    "[{}] {:3}. {} - {} ({})",
                    account_label,
                    index + 1,
                    album.title,
                    album.label,
                    album.id
                );
            }
        } else if let Err(e) = downloader.sync_all_albums(albums).await {
            failures.push(format!("{account_label}: {e}"));
        }
    }

    if requested_album_id.is_some() && !requested_album_found {
        failures.push("所有账号均未找到或无法访问指定专辑".to_string());
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(failures.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::default();
        assert!(config.accounts().is_empty());
        assert_eq!(config.download.formats.len(), 2);
        assert!(config.download.formats.contains(&"320".to_string()));
        assert!(config.download.formats.contains(&"FLAC".to_string()));
    }
}
