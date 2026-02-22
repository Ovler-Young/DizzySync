use crate::client::DiscInfo;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn extract_year_from_date(date_str: &str) -> Option<String> {
    if let Some(caps) = regex::Regex::new(r"(\d{4})年").unwrap().captures(date_str) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    if let Some(caps) = regex::Regex::new(r"^(\d{4})[/-]")
        .unwrap()
        .captures(date_str)
    {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    None
}

pub fn generate_readme(disc_info: &DiscInfo, album_dir: &Path, formats: &[String]) -> Result<()> {
    let template = load_readme_template().unwrap_or_else(|_| get_default_readme_template());
    let content = apply_template_variables(&template, disc_info, formats);
    fs::write(album_dir.join("README.md"), content)?;
    Ok(())
}

pub fn generate_nfo(disc_info: &DiscInfo, album_dir: &Path) -> Result<()> {
    let content = generate_nfo_content(disc_info);
    fs::write(album_dir.join("album.nfo"), content)?;
    Ok(())
}

fn load_readme_template() -> Result<String> {
    Ok(fs::read_to_string("readme.template.md")?)
}

fn get_default_readme_template() -> String {
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
"#
    .to_string()
}

pub fn apply_template_variables(
    template: &str,
    disc_info: &DiscInfo,
    formats: &[String],
) -> String {
    let mut result = template.to_string();

    result = result.replace("{album}", &disc_info.title);
    result = result.replace("{label}", &disc_info.label);
    result = result.replace("{id}", &disc_info.id);
    result = result.replace("{cover}", &disc_info.cover);
    result = result.replace(
        "{release_date}",
        disc_info.release_date.as_deref().unwrap_or("未知"),
    );
    let description = disc_info.disc_description.as_deref().unwrap_or("暂无描述");
    result = result.replace("{description}", description);
    result = result.replace("{tags}", &disc_info.tags.join(", "));
    let authors = disc_info
        .tracks
        .first()
        .map(|t| t.authers.as_str())
        .unwrap_or(&disc_info.label);
    result = result.replace("{authors}", authors);
    let year = disc_info
        .release_date
        .as_deref()
        .and_then(extract_year_from_date)
        .unwrap_or_else(|| "未知".to_string());
    result = result.replace("{year}", &year);
    result = result.replace(
        "{download_date}",
        &chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    );
    result = result.replace("{formats}", &formats.join(", "));

    result
}

pub fn generate_nfo_content(disc_info: &DiscInfo) -> String {
    let authors = disc_info
        .tracks
        .first()
        .map(|t| t.authers.as_str())
        .unwrap_or(&disc_info.label);
    let year = disc_info
        .release_date
        .as_deref()
        .and_then(extract_year_from_date)
        .unwrap_or_else(|| "Unknown".to_string());

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
        disc_info.title,
        authors,
        disc_info
            .tags
            .first()
            .map(|s| s.as_str())
            .unwrap_or("Music"),
        year,
        disc_info.release_date.as_deref().unwrap_or("Unknown"),
        disc_info.label,
        disc_info.id,
        disc_info.disc_description.as_deref().unwrap_or(""),
        disc_info
            .tags
            .iter()
            .map(|tag| format!("        <tag>{tag}</tag>"))
            .collect::<Vec<_>>()
            .join("\n"),
        disc_info.id
    )
}
