use std::sync::Arc;

use anyhow::{Result, anyhow};
use libcalibre::{
    UpsertBookIdentifier,
    client::CalibreClient,
    dtos::{
        author::NewAuthorDto,
        book::NewBookDto,
        language::NewLanguageDto,
        library::{NewLibraryEntryDto, NewLibraryFileDto},
        publisher::NewPublisherDto,
        rating::NewRatingDto,
        tag::NewTagDto,
    },
};
use libeh::dto::keyword::Keyword;
use log::info;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::Mutex;

use super::{Gallery, parse_category, parse_category_str, parse_tag};
use crate::g_info;
use crate::tag_db::db::EhTagDb;

static TITLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:\([^\(\)]+\))?\s*(?:\[[^\[\]]+\])?\s*([^\[\]\(\)]+)").unwrap());

pub async fn add_to_calibre(
    calibre_client: Arc<Mutex<CalibreClient>>,
    tag_db: Arc<Mutex<EhTagDb>>,
    is_exhentai: bool,
    cbz_path: &str,
    gallery: &Gallery,
    gid_token: &str,
) -> Result<()> {
    let mut client = calibre_client.lock().await;

    let gallery_title;
    let gallery_title_jpn;
    let gallery_category;
    let gallery_gid;
    let gallery_token;
    let gallery_rating;
    let gallery_tags;

    match gallery {
        Gallery::Detail(detail) => {
            gallery_title = &detail.info.title;
            gallery_title_jpn = &detail.info.title_jpn;
            gallery_category = parse_category(&detail.info.category);
            gallery_gid = detail.info.gid;
            gallery_token = &detail.info.token;
            gallery_rating = &detail.info.rating;
            gallery_tags = &detail.info.tags;
        }
        Gallery::Metadata(metadata) => {
            gallery_title = &metadata.title;
            gallery_title_jpn = &metadata.title_jpn;
            gallery_category = parse_category_str(&metadata.category);
            gallery_gid = metadata.gid;
            gallery_token = &metadata.token;
            gallery_rating = &metadata.rating;
            gallery_tags = &metadata.tags;
        }
    };

    let title = if !gallery_title_jpn.is_empty() {
        gallery_title_jpn
    } else {
        gallery_title
    };
    let title = TITLE_REGEX
        .captures(title)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim())
        .unwrap_or(gallery_title);
    let book_dto = NewBookDto {
        title: title.to_string(),
        timestamp: None,
        pubdate: None,
        series_index: 1.0,
        flags: 1,
        has_cover: None,
    };

    let category_name = match gallery_category {
        Some(c) => Some(
            tag_db
                .lock()
                .await
                .get_tag_name("reclass", c)?
                .unwrap_or(c.to_string()),
        ),
        None => None,
    };

    let identifiers_dto = vec![UpsertBookIdentifier {
        book_id: 0,
        id: None,
        label: "ehentai".to_string(),
        value: format!(
            "{}_{}_{}",
            gallery_gid,
            gallery_token,
            if is_exhentai { 1 } else { 0 }
        ),
    }];

    let rating_dto = Some(NewRatingDto {
        rating: (gallery_rating * 2.0).floor() as i32,
    });

    let files_dto = Some(vec![NewLibraryFileDto {
        path: cbz_path.into(),
    }]);

    let mut authors_dto: Vec<NewAuthorDto> = Vec::new();
    let mut publishers_dto: Vec<NewPublisherDto> = Vec::new();
    let mut language_dto: Option<NewLanguageDto> = None;
    let mut tags_dto: Vec<NewTagDto> = Vec::new();

    for tag in gallery_tags.iter() {
        let result = parse_tag(tag);
        if result.is_none() {
            continue;
        }
        let (namespace, raw_tag) = result.unwrap();
        let tag_namespace = tag_db
            .lock()
            .await
            .get_tag_name("rows", namespace)?
            .unwrap_or(namespace.to_string());
        let tag_name = tag_db
            .lock()
            .await
            .get_tag_name(namespace, raw_tag)?
            .unwrap_or(raw_tag.to_string());
        match tag {
            Keyword::Artist(_) => {
                let author_dto = NewAuthorDto {
                    full_name: tag_name,
                    sortable_name: String::new(),
                    external_url: None,
                };
                authors_dto.push(author_dto);
            }
            Keyword::Group(_) => {
                let publisher_dto = NewPublisherDto {
                    name: tag_name,
                    sort: None,
                };
                publishers_dto.push(publisher_dto);
            }
            Keyword::Language(_) => {
                language_dto = Some(NewLanguageDto {
                    lang_code: raw_tag.to_string(),
                });
                let tag_dto = NewTagDto {
                    name: format!("{}:{}", tag_namespace, tag_name),
                };
                tags_dto.push(tag_dto);
            }
            _ => {
                let tag_dto = NewTagDto {
                    name: format!("{}:{}", tag_namespace, tag_name),
                };
                tags_dto.push(tag_dto);
            }
        }
    }

    let authors_dto = if authors_dto.is_empty() {
        vec![NewAuthorDto {
            full_name: "Unknown".to_string(),
            sortable_name: String::new(),
            external_url: None,
        }]
    } else {
        authors_dto
    };

    let publishers_dto = if publishers_dto.is_empty() {
        vec![NewPublisherDto {
            name: "Unknown".to_string(),
            sort: None,
        }]
    } else {
        publishers_dto
    };

    let language_dto = language_dto.or_else(|| {
        Some(NewLanguageDto {
            lang_code: "jpn".to_string(),
        })
    });

    if category_name.is_some() {
        let tag_dto = NewTagDto {
            name: format!("分类:{}", category_name.unwrap()),
        };
        tags_dto.push(tag_dto);
    }

    let dto = NewLibraryEntryDto {
        book: book_dto,
        authors: authors_dto,
        publishers: publishers_dto,
        identifiers: identifiers_dto,
        language: language_dto,
        tags: tags_dto,
        rating: rating_dto,
        files: files_dto,
    };

    g_info!(gid_token, "Adding book to calibre");
    client.add_book(dto).map_err(|e| anyhow!("{}", e))?;

    Ok(())
}
