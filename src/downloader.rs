use crate::client::{Album, DizzylabClient};
use crate::config::Config;
use anyhow::{anyhow, Result};
use chrono::{self, Datelike};
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use zip::ZipArchive;
use unrar::Archive;
use encoding_rs::GBK;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy)]
enum ArchiveFormat {
    Zip,
    Rar,
    Unknown,
}

pub struct Downloader {
    client: DizzylabClient,
    config: Config,
}

impl Downloader {
    pub fn new(client: DizzylabClient, config: Config) -> Self {
        Self { client, config }
    }

    pub async fn sync_all_albums(&self, mut albums: Vec<Album>) -> Result<()> {
        let total_albums = albums.len();
        info!("开始同步 {} 个专辑", total_albums);

        // 创建主输出目录
        fs::create_dir_all(&self.config.paths.output_dir)?;

        for (index, album) in albums.iter_mut().enumerate() {
            info!(
                "处理专辑 {}/{}: {} - {}",
                index + 1,
                total_albums,
                album.title,
                album.label
            );

            if album.release_date.is_none() {
                album.release_date = None;
                album.description = None;
                album.tags = Vec::new();
                album.year = None;
                album.authors = None;
            }

            if let Err(e) = self.download_album(album).await {
                error!("下载专辑 {} 失败: {}", album.id, e);
                continue;
            }

            // 单线程模式，添加延迟
            if self.config.behavior.single_threaded {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }

        info!("同步完成！");
        Ok(())
    }

    async fn download_album(&self, album: &mut Album) -> Result<()> {
        // 获取专辑详细信息
        if let Err(e) = self.client.get_album_details(album).await {
            warn!("获取专辑 {} 详细信息失败: {}", album.id, e);
            // 继续处理，即使没有详细信息
        }

        // 创建专辑目录
        let album_dir = self.get_album_directory(album);
        
        if self.config.behavior.skip_existing && album_dir.exists() {
            info!("专辑目录已存在，跳过: {}", album.title);
            return Ok(());
        }

        info!("album_dir: {}", album_dir.display());

        fs::create_dir_all(&album_dir)?;

        // 生成README和NFO文件
        if self.config.behavior.generate_readme {
            if let Err(e) = self.generate_readme(album, &album_dir).await {
                warn!("生成README失败: {}", e);
            }
        }

        if self.config.behavior.generate_nfo {
            if let Err(e) = self.generate_nfo(album, &album_dir).await {
                warn!("生成NFO失败: {}", e);
            }
        }

        // 如果只下载元数据，跳过音频文件下载
        if self.config.behavior.metadata_only {
            info!("仅下载元数据模式：跳过音频文件下载 - {}", album.title);
            return Ok(());
        }

        // 下载每种格式
        for format in &self.config.download.formats {
            if let Err(e) = self.download_format(album, format, &album_dir).await {
                warn!("下载格式 {} 失败: {}", format, e);
                // 继续下载其他格式，不要因为一个格式失败就停止
                continue;
            }
        }

        Ok(())
    }

    async fn download_format(&self, album: &Album, format: &str, album_dir: &PathBuf) -> Result<()> {
        info!("下载格式: {} - {}", album.title, format);

        // 获取下载链接
        let download_links = self.client.get_download_links(&album.id, format).await?;
        
        // 检查是否有有效的下载链接（主要针对gift格式）
        if download_links.is_empty() {
            info!("专辑 {} 没有 {} 格式，跳过", album.title, format);
            return Ok(());
        }
        
        let download_url = download_links
            .get(format)
            .ok_or_else(|| anyhow!("无法获取格式 {} 的下载链接", format))?;

        // 下载文件
        let file_data = self.client.download_file(download_url, &album.id).await?;

        // 检查压缩文件格式并解压
        let archive_format = self.detect_archive_format(&file_data);
        match archive_format {
            ArchiveFormat::Zip => {
                self.extract_zip_file(&file_data, album, format, album_dir)?;
            }
            ArchiveFormat::Rar => {
                self.extract_rar_file(&file_data, album, format, album_dir)?;
            }
            ArchiveFormat::Unknown => {
                // 直接保存文件
                let filename = format!("{}.{}", album.title, self.get_file_extension(format));
                let file_path = album_dir.join(filename);
                let mut file = File::create(file_path)?;
                file.write_all(&file_data)?;
            }
        }

        Ok(())
    }

    fn extract_zip_file(
        &self,
        zip_data: &[u8],
        _album: &Album,
        format: &str,
        album_dir: &PathBuf,
    ) -> Result<()> {
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            
            // 使用 name_raw() 获取原始字节，然后尝试解码
            let file_name_raw = file.name_raw();
            let file_name: Cow<str> = match std::str::from_utf8(file_name_raw) {
                Ok(name) => Cow::Borrowed(name),
                Err(_) => GBK.decode(file_name_raw).0,
            };

            // 跳过目录
            if file_name.ends_with('/') {
                continue;
            }

            debug!("解压文件: {}", file_name);

            let output_path = if self.config.download.flatten {
                // 铺平模式：直接放在专辑目录下，不创建格式子文件夹
                album_dir.join(&*file_name)
            } else {
                // 格式子文件夹模式：根据格式创建子目录
                let format_dir = album_dir.join(format);
                fs::create_dir_all(&format_dir)?;
                format_dir.join(&*file_name)
            };

            // 确保输出目录存在
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut output_file = File::create(&output_path)?;
            std::io::copy(&mut file, &mut output_file)?;
        }

        Ok(())
    }

    fn detect_archive_format(&self, data: &[u8]) -> ArchiveFormat {
        if data.len() < 4 {
            return ArchiveFormat::Unknown;
        }

        // 检查ZIP格式
        if data.starts_with(b"PK") {
            return ArchiveFormat::Zip;
        }

        // 检查RAR格式
        // RAR5格式的魔数
        if data.len() >= 8 && &data[0..8] == b"Rar!\x1a\x07\x01\x00" {
            return ArchiveFormat::Rar;
        }
        // RAR4格式的魔数
        if data.len() >= 7 && &data[0..7] == b"Rar!\x1a\x07\x00" {
            return ArchiveFormat::Rar;
        }

        ArchiveFormat::Unknown
    }

    fn extract_rar_file(
        &self,
        rar_data: &[u8],
        album: &Album,
        format: &str,
        album_dir: &PathBuf,
    ) -> Result<()> {
        // 创建临时文件来存储RAR数据
        let temp_file_path = album_dir.join(format!("temp_{}.rar", album.id));
        
        // 写入临时文件
        fs::write(&temp_file_path, rar_data)?;
        
        // 使用unrar库解压
        let archive = Archive::new(&temp_file_path);
        let archive = archive.open_for_processing()?;
        
        // 递归处理所有文件
        self.process_rar_archive(archive, format, album_dir)?;
        
        // 删除临时文件
        if let Err(e) = fs::remove_file(&temp_file_path) {
            warn!("删除临时RAR文件失败: {}", e);
        }
        
        Ok(())
    }

    fn process_rar_archive(
        &self,
        mut archive: unrar::OpenArchive<unrar::Process, unrar::CursorBeforeHeader>,
        format: &str,
        album_dir: &PathBuf,
    ) -> Result<()> {
        loop {
            match archive.read_header() {
                Ok(Some(header_archive)) => {
                    let entry = header_archive.entry();
                    let filename = &entry.filename;
                    
                    // 跳过目录
                    if entry.is_directory() {
                        archive = header_archive.skip()?;
                        continue;
                    }

                    debug!("解压RAR文件: {}", filename.display());

                    let output_path = if self.config.download.flatten {
                        // 铺平模式：直接放在专辑目录下
                        album_dir.join(filename)
                    } else {
                        // 格式子文件夹模式：根据格式创建子目录
                        let format_dir = album_dir.join(format);
                        fs::create_dir_all(&format_dir)?;
                        format_dir.join(filename)
                    };

                    // 确保输出目录存在
                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // 解压文件
                    let (data, next_archive) = header_archive.read()?;
                    fs::write(&output_path, data)?;
                    
                    archive = next_archive;
                }
                Ok(None) => {
                    // 没有更多文件
                    break;
                }
                Err(e) => {
                    error!("读取RAR头部失败: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }

    fn get_album_directory(&self, album: &Album) -> PathBuf {
        // 使用目录名模板生成路径
        let directory_name = self.generate_directory_name(album);
        self.config.paths.output_dir.join(directory_name)
    }

    fn generate_directory_name(&self, album: &Album) -> String {
        let template = &self.config.paths.directory_template;
        
        // 获取当前年份（如果需要的话）
        let current_year = chrono::Utc::now().year().to_string();
        let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        
        // 替换模板变量
        let mut result = template.clone();
        result = result.replace("{album}", &self.sanitize_filename(&album.title));
        result = result.replace("{label}", &self.sanitize_filename(&album.label));
        
        // 使用专辑的作者信息，如果没有则使用厂牌名
        let authors = album.authors.as_ref()
            .unwrap_or(&album.label);
        result = result.replace("{authors}", &self.sanitize_filename(authors));
        
        // 使用专辑的年份，如果没有则使用当前年份
        let year = album.year.as_ref()
            .unwrap_or(&current_year);
        result = result.replace("{year}", year);
        
        // 使用专辑的发布日期，如果没有则使用当前日期
        let date = album.release_date.as_ref()
            .map(|d| self.convert_chinese_date_to_iso(d))
            .unwrap_or(current_date);
        result = result.replace("{date}", &date);
        
        result
    }

    fn convert_chinese_date_to_iso(&self, chinese_date: &str) -> String {
        // 将"2025年6月10日"格式转换为"2025-06-10"格式
        if let Some(captures) = regex::Regex::new(r"(\d{4})年(\d{1,2})月(\d{1,2})日").unwrap().captures(chinese_date) {
            if let (Some(year), Some(month), Some(day)) = (captures.get(1), captures.get(2), captures.get(3)) {
                return format!("{}-{:02}-{:02}", 
                    year.as_str(), 
                    month.as_str().parse::<u32>().unwrap_or(1),
                    day.as_str().parse::<u32>().unwrap_or(1)
                );
            }
        }
        chinese_date.to_string()
    }

    fn sanitize_filename(&self, name: &str) -> String {
        // 移除或替换文件系统不支持的字符
        name.chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn get_file_extension(&self, format: &str) -> &str {
        match format {
            "128" => "mp3",
            "MP3" => "mp3",
            "FLAC" => "flac",
            "gift" => "unknown", // gift格式可能是ZIP或RAR，让自动检测处理
            _ => "bin",
        }
    }

    async fn generate_readme(&self, album: &Album, album_dir: &PathBuf) -> Result<()> {
        let template_content = self.load_readme_template().unwrap_or_else(|_| self.get_default_readme_template());
        let readme_content = self.apply_template_variables(&template_content, album);
        
        let readme_path = album_dir.join("README.md");
        fs::write(readme_path, readme_content)?;
        
        Ok(())
    }

    async fn generate_nfo(&self, album: &Album, album_dir: &PathBuf) -> Result<()> {
        let nfo_content = self.generate_nfo_content(album);
        let nfo_path = album_dir.join("album.nfo");
        fs::write(nfo_path, nfo_content)?;
        
        Ok(())
    }

    fn load_readme_template(&self) -> Result<String> {
        Ok(fs::read_to_string("readme.template.md")?)
    }

    fn get_default_readme_template(&self) -> String {
        r#"# {album}

**厂牌:** {label}  
**发布日期:** {release_date}  
**专辑ID:** {id}  

## 描述

{description}

## 标签

{tags}

## 下载信息

- **下载时间:** {download_date}
- **下载格式:** {formats}

---

*由 DizzySync 自动生成*
"#.to_string()
    }

    fn apply_template_variables(&self, template: &str, album: &Album) -> String {
        let mut result = template.to_string();
        
        result = result.replace("{album}", &album.title);
        result = result.replace("{label}", &album.label);
        result = result.replace("{id}", &album.id);
        result = result.replace("{cover}", &album.cover);
        result = result.replace("{release_date}", 
            album.release_date.as_ref().unwrap_or(&"未知".to_string()));
        result = result.replace("{description}", 
            album.description.as_ref().unwrap_or(&"暂无描述".to_string()));
        result = result.replace("{tags}", &album.tags.join(", "));
        result = result.replace("{authors}", 
            album.authors.as_ref().unwrap_or(&album.label));
        result = result.replace("{year}", 
            album.year.as_ref().unwrap_or(&"未知".to_string()));
        result = result.replace("{download_date}", 
            &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        result = result.replace("{formats}", &self.config.download.formats.join(", "));
        
        result
    }

    fn generate_nfo_content(&self, album: &Album) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<album>
    <title>{}</title>
    <artist>{}</artist>
    <genre>{}</genre>
    <year>{}</year>
    <releasedate>{}</releasedate>
    <label>{}</label>
    <id>{}</id>
    <plot>{}</plot>
    <tags>
        {}
    </tags>
    <source>Dizzylab</source>
    <url>https://www.dizzylab.net/d/{}/</url>
</album>"#,
            album.title,
            album.authors.as_ref().unwrap_or(&album.label),
            album.tags.get(0).unwrap_or(&"Music".to_string()),
            album.year.as_ref().unwrap_or(&"Unknown".to_string()),
            album.release_date.as_ref().unwrap_or(&"Unknown".to_string()),
            album.label,
            album.id,
            album.description.as_ref().unwrap_or(&"".to_string()),
            album.tags.iter().map(|tag| format!("        <tag>{}</tag>", tag)).collect::<Vec<_>>().join("\n"),
            album.id
        )
    }
} 