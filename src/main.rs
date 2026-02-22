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

    if !Path::new(config_path).exists() {
        error!("配置文件不存在: {}", config_path);
        error!("请运行 'dizzysync --init' 创建默认配置文件");
        return Ok(());
    }

    let mut config = Config::load_from_file(config_path)?;

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
        config.paths.output_dir = PathBuf::from(output_dir);
        info!("设置输出目录: {}", output_dir);
    }

    if config.user.username.is_empty() || config.user.password.is_empty() {
        error!("请在配置文件中设置 username 和 password");
        return Ok(());
    }

    // Create client and login
    let client = DizzylabClient::new(config.behavior.debug)?;
    let token = match client
        .login(&config.user.username, &config.user.password)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            error!("登录失败: {}", e);
            return Ok(());
        }
    };

    // Get user info (for display only)
    if let Ok(user_info) = client.get_my_info(&token).await {
        info!("已登录为: {} (UID: {})", user_info.username, user_info.uid);
    }

    // Build album list
    let albums = if let Some(album_id) = matches.get_one::<String>("id") {
        info!("获取指定专辑: {}", album_id);
        match client.get_disc_info(album_id, &token).await {
            Ok(disc_info) => {
                // Wrap as a DiscListItem so sync_all_albums can fetch details again
                // (or we can download directly)
                let downloader = Downloader::new(client, config, token);
                if matches.get_flag("dry-run") {
                    println!(
                        "  1. {} - {} ({})",
                        disc_info.title, disc_info.label, disc_info.id
                    );
                } else {
                    downloader.download_album(&disc_info).await?;
                }
                return Ok(());
            }
            Err(e) => {
                error!("获取专辑 {} 失败: {}", album_id, e);
                return Ok(());
            }
        }
    } else {
        client.get_my_discs(&token).await?
    };

    if albums.is_empty() {
        info!("没有找到任何专辑");
        return Ok(());
    }

    info!("找到 {} 个专辑", albums.len());

    if matches.get_flag("dry-run") {
        info!("=== 专辑列表 ===");
        for (index, album) in albums.iter().enumerate() {
            println!(
                "{:3}. {} - {} ({})",
                index + 1,
                album.title,
                album.label,
                album.id
            );
        }
        return Ok(());
    }

    let downloader = Downloader::new(client, config, token);
    downloader.sync_all_albums(albums).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::default();
        assert!(config.user.username.is_empty());
        assert!(config.user.password.is_empty());
        assert_eq!(config.download.formats.len(), 2);
        assert!(config.download.formats.contains(&"320".to_string()));
        assert!(config.download.formats.contains(&"FLAC".to_string()));
    }
}
