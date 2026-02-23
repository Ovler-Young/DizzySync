use crate::types::DiscInfo;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Normalize a Dizzylab date string to "YYYY-MM-DD".
/// Handles "2023年4月1日" and "2023-04-01" / "2023/04/01".
/// Returns the original string unchanged if no pattern matches.
pub fn normalize_date(date_str: &str) -> String {
    if let Some(caps) = regex::Regex::new(r"(\d{4})年(\d{1,2})月(\d{1,2})日")
        .unwrap()
        .captures(date_str)
    {
        if let (Some(y), Some(m), Some(d)) = (caps.get(1), caps.get(2), caps.get(3)) {
            return format!(
                "{}-{:02}-{:02}",
                y.as_str(),
                m.as_str().parse::<u32>().unwrap_or(1),
                d.as_str().parse::<u32>().unwrap_or(1),
            );
        }
    }
    date_str.to_string()
}

pub fn extract_year_from_date(date_str: &str) -> Option<String> {
    let normalized = normalize_date(date_str);
    if let Some(caps) = regex::Regex::new(r"^(\d{4})[/-]")
        .unwrap()
        .captures(&normalized)
    {
        return caps.get(1).map(|m| m.as_str().to_string());
    }
    None
}

fn format_price(price: &serde_json::Value) -> String {
    let v = match price {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => return "未知".to_string(),
    };
    if v == 0.0 {
        "免费".to_string()
    } else if v == 999.0 {
        "仅兑换可得".to_string()
    } else {
        format!("¥{v}")
    }
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
**价格:** {price}
**状态:** {status_flags}

## 描述

{description}

{description_2}

## 厂牌介绍

{label_description}

## 曲目列表

{tracklist}

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

    let description_2 = match disc_info.disc_description_2.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => String::new(),
    };
    result = result.replace("{description_2}", &description_2);

    let label_description = match disc_info.label_description.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => "暂无厂牌介绍".to_string(),
    };
    result = result.replace("{label_description}", &label_description);

    let price_str = disc_info
        .price
        .as_ref()
        .map(format_price)
        .unwrap_or_else(|| "未知".to_string());
    result = result.replace("{price}", &price_str);

    let mut flags = Vec::new();
    if disc_info.onsell {
        flags.push("在售");
    }
    if disc_info.ispreselling {
        flags.push("预售中");
    }
    if disc_info.hasgift {
        flags.push("含特典");
    }
    if disc_info.onlyhavegift {
        flags.push("仅特典赠送");
    }
    let status_flags = if flags.is_empty() {
        "—".to_string()
    } else {
        flags.join(" / ")
    };
    result = result.replace("{status_flags}", &status_flags);

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

    let tracklist = disc_info
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let author = if t.authers.is_empty() {
                &disc_info.label
            } else {
                &t.authers
            };
            format!("{}. {} — {}", i + 1, t.title, author)
        })
        .collect::<Vec<_>>()
        .join("\n");
    result = result.replace("{tracklist}", &tracklist);

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
    let price_str = disc_info
        .price
        .as_ref()
        .map(format_price)
        .unwrap_or_else(|| "Unknown".to_string());

    let tracks_xml = disc_info
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let author = if t.authers.is_empty() {
                disc_info.label.as_str()
            } else {
                t.authers.as_str()
            };
            format!(
                "        <track>\n            <position>{}</position>\n            \
                 <title>{}</title>\n            <artist>{}</artist>\n            \
                 <id>{}</id>\n        </track>",
                i + 1,
                t.title,
                author,
                t.id,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let tags_xml = disc_info
        .tags
        .iter()
        .map(|tag| format!("        <tag>{tag}</tag>"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<album>
    <title>{title}</title>
    <artist>{artist}</artist>
    <genre>{genre}</genre>
    <year>{year}</year>
    <releasedate>{releasedate}</releasedate>
    <label>{label}</label>
    <label_description>{label_description}</label_description>
    <id>{id}</id>
    <price>{price}</price>
    <onsell>{onsell}</onsell>
    <ispreselling>{ispreselling}</ispreselling>
    <hasgift>{hasgift}</hasgift>
    <onlyhavegift>{onlyhavegift}</onlyhavegift>
    <plot>{plot}</plot>
    <plot2>{plot2}</plot2>
    <tags>
{tags_xml}
    </tags>
    <tracklist>
{tracks_xml}
    </tracklist>
    <source>Dizzylab</source>
    <url>https://www.dizzylab.net/d/{id}/</url>
</album>"#,
        title = disc_info.title,
        artist = authors,
        genre = disc_info
            .tags
            .first()
            .map(|s| s.as_str())
            .unwrap_or("Music"),
        year = year,
        releasedate = disc_info.release_date.as_deref().unwrap_or("Unknown"),
        label = disc_info.label,
        label_description = disc_info.label_description.as_deref().unwrap_or(""),
        id = disc_info.id,
        price = price_str,
        onsell = disc_info.onsell,
        ispreselling = disc_info.ispreselling,
        hasgift = disc_info.hasgift,
        onlyhavegift = disc_info.onlyhavegift,
        plot = disc_info.disc_description.as_deref().unwrap_or(""),
        plot2 = disc_info.disc_description_2.as_deref().unwrap_or(""),
        tags_xml = tags_xml,
        tracks_xml = tracks_xml,
    )
}
