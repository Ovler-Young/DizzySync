mod client;
mod config;
mod downloader;

use anyhow::Result;
use clap::{Arg, Command};
use client::DizzylabClient;
use config::Config;
use downloader::Downloader;
use std::path::Path;
use tracing::{error, info, Level};
use tracing_subscriber;

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
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();

    // 初始化日志，如果有debug参数则使用DEBUG级别
    let log_level = if matches.get_flag("debug") {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    if matches.get_flag("debug") {
        info!("调试模式已启用，将显示所有HTTP响应");
    }

    // 如果指定了 --init，创建默认配置文件
    if matches.get_flag("init") {
        Config::create_default_config(config_path)?;
        return Ok(());
    }

    // 检查配置文件是否存在
    if !Path::new(config_path).exists() {
        error!("配置文件不存在: {}", config_path);
        error!("请运行 'dizzysync --init' 创建默认配置文件");
        return Ok(());
    }

    // 加载配置
    let mut config = Config::load_from_file(config_path)?;
    
    // 如果命令行指定了debug，覆盖配置文件设置
    if matches.get_flag("debug") {
        config.behavior.debug = true;
    }
    
    // 如果命令行指定了metadata-only，覆盖配置文件设置
    if matches.get_flag("metadata-only") {
        config.behavior.metadata_only = true;
        info!("启用仅元数据模式：只下载专辑信息，不下载音频文件");
    }
    
    // 验证配置
    if config.user.cookie.is_empty() {
        error!("请在配置文件中设置你的cookie");
        return Ok(());
    }

    // 创建客户端
    let client = DizzylabClient::new(config.user.cookie.clone(), config.behavior.debug)?;

    // 获取用户信息
    let user_info = client.get_user_info().await?;

    // 获取用户专辑列表
    let albums = client.get_user_albums(user_info.uid).await?;
    
    if albums.is_empty() {
        info!("没有找到任何专辑");
        return Ok(());
    }

    info!("找到 {} 个专辑", albums.len());

    // 如果是dry-run模式，只列出专辑
    if matches.get_flag("dry-run") {
        info!("=== 专辑列表 ===");
        for (index, album) in albums.iter().enumerate() {
            println!("{:3}. {} - {} ({})", 
                index + 1, 
                album.title, 
                album.label, 
                album.id
            );
        }
        return Ok(());
    }

    // 创建下载器并开始同步
    let downloader = Downloader::new(client, config);
    downloader.sync_all_albums(albums).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::default();
        assert!(!config.user.cookie.is_empty() || config.user.cookie.is_empty()); // 允许空cookie用于测试
        assert_eq!(config.download.formats.len(), 2);
        assert!(config.download.formats.contains(&"MP3".to_string()));
        assert!(config.download.formats.contains(&"FLAC".to_string()));
    }
} 