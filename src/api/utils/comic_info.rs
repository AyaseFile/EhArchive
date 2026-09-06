use std::io::Cursor;

use anyhow::Result;
use libeh::dto::{api::GalleryMetadata, keyword::Keyword};
use once_cell::sync::Lazy;
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use regex::Regex;

use super::{parse_category, parse_tag};
use crate::tag_db::db::EhTagDb;

const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/anansi-project/comicinfo/main/drafts/v2.1/ComicInfo.xsd";

static TITLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:\([^\(\)]+\))?\s*(?:\[[^\[\]]+\])?\s*([^\[\]\(\)]+)").unwrap());

#[derive(Debug)]
pub struct ComicMetadata {
    pub title: String,
    pub web: String,
    pub tags: Vec<String>,
    pub pencillers: Vec<String>,
    pub publisher: Option<String>,
    pub language_iso: Option<String>,
    pub community_rating: String,
}

pub fn gallery_to_comic_metadata(
    site: &str,
    metadata: &GalleryMetadata,
    tag_db: &mut EhTagDb,
) -> Result<ComicMetadata> {
    let preferred = if metadata.title_jpn.is_empty() {
        &metadata.title
    } else {
        &metadata.title_jpn
    };
    let title = clean_title(preferred, &metadata.title);
    let mut tags = Vec::new();
    let mut pencillers = Vec::new();
    let mut publisher = None;
    let mut language_iso = None;

    for tag in &metadata.tags {
        let Some((namespace, raw_tag)) = parse_tag(tag) else {
            continue;
        };
        let tag_namespace = tag_db
            .get_tag_name("rows", namespace)?
            .unwrap_or_else(|| namespace.to_string());
        let tag_name = tag_db
            .get_tag_name(namespace, raw_tag)?
            .unwrap_or_else(|| raw_tag.to_string());
        match tag {
            Keyword::Artist(_) => pencillers.push(tag_name),
            Keyword::Group(_) => publisher = Some(tag_name),
            Keyword::Language(_) => {
                if let Some(language) = language_code(raw_tag) {
                    language_iso = Some(language.to_string());
                }
                tags.push(format!("{tag_namespace}:{tag_name}"));
            }
            _ => tags.push(format!("{tag_namespace}:{tag_name}")),
        }
    }

    if let Some(category) = parse_category(metadata.category.clone()) {
        let category = tag_db
            .get_tag_name("reclass", &category)?
            .unwrap_or(category);
        tags.push(format!("分类:{category}"));
    }

    Ok(ComicMetadata {
        title,
        web: format!("https://{site}/g/{}/{}/", metadata.gid, metadata.token),
        tags,
        pencillers,
        publisher,
        language_iso: language_iso.or_else(|| Some("ja".to_string())),
        community_rating: format_rating(metadata.rating),
    })
}

fn clean_title(preferred: &str, fallback: &str) -> String {
    TITLE_REGEX
        .captures(preferred)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn language_code(raw: &str) -> Option<&'static str> {
    match raw.to_ascii_lowercase().as_str() {
        "japanese" => Some("ja"),
        "chinese" => Some("zh"),
        "english" => Some("en"),
        "spanish" => Some("es"),
        "korean" => Some("ko"),
        "speechless" => Some("zxx"),
        _ => None,
    }
}

fn format_rating(rating: f32) -> String {
    format!("{rating:.1}")
}

pub fn serialize_comic_info(metadata: &ComicMetadata) -> Result<Vec<u8>> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut root = BytesStart::new("ComicInfo");
    root.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    root.push_attribute(("xsi:noNamespaceSchemaLocation", SCHEMA_URL));
    writer.write_event(Event::Start(root))?;
    write_element(&mut writer, "Title", &metadata.title)?;
    if !metadata.pencillers.is_empty() {
        write_element(&mut writer, "Penciller", &metadata.pencillers.join(", "))?;
    }
    if let Some(value) = &metadata.publisher {
        write_element(&mut writer, "Publisher", value)?;
    }
    if !metadata.tags.is_empty() {
        write_element(&mut writer, "Tags", &metadata.tags.join(", "))?;
    }
    write_element(&mut writer, "Web", &metadata.web)?;
    if let Some(value) = &metadata.language_iso {
        write_element(&mut writer, "LanguageISO", value)?;
    }
    write_element(&mut writer, "CommunityRating", &metadata.community_rating)?;
    writer.write_event(Event::End(BytesEnd::new("ComicInfo")))?;
    Ok(writer.into_inner().into_inner())
}

fn write_element(writer: &mut Writer<Cursor<Vec<u8>>>, name: &str, value: &str) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(value)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}
