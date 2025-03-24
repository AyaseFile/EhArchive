use std::sync::Arc;

use anyhow::{Error, Result};
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
use libeh::dto::{gallery::detail::GalleryDetail, keyword::Keyword};
use log::info;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::Mutex;

use super::{parse_category, parse_tag};
use crate::g_info;
use crate::tag_db::db::EhTagDb;

static TITLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:\([^\(\)]+\))?\s*(?:\[[^\[\]]+\])?\s*([^\[\]\(\)]+)").unwrap());

pub async fn add_to_calibre(
    calibre_client: Arc<Mutex<CalibreClient>>,
    tag_db: Arc<Mutex<EhTagDb>>,
    is_exhentai: bool,
    cbz_path: String,
    detail: GalleryDetail,
    gid_token: String,
) -> Result<()> {
    let mut client = calibre_client.lock().await;

    let title = if !detail.info.title_jpn.is_empty() {
        &detail.info.title_jpn
    } else {
        &detail.info.title
    };
    let title = TITLE_REGEX
        .captures(title)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim())
        .unwrap_or(&detail.info.title);
    let book_dto = NewBookDto {
        title: title.to_string(),
        timestamp: None,
        pubdate: None,
        series_index: 1.0,
        flags: 1,
        has_cover: None,
    };

    let category = parse_category(&detail.info.category);
    let category_name = match category {
        Some(c) => Some(
            tag_db
                .lock()
                .await
                .get_tag_name("reclass", category.unwrap())?
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
            detail.info.gid,
            detail.info.token,
            if is_exhentai { 1 } else { 0 }
        ),
    }];
    let rating_dto = Some(NewRatingDto {
        rating: (detail.info.rating * 2.0).floor() as i32,
    });
    let files_dto = Some(vec![NewLibraryFileDto {
        path: cbz_path.into(),
    }]);

    let mut authors_dto: Vec<NewAuthorDto> = Vec::new();
    let mut publishers_dto: Vec<NewPublisherDto> = Vec::new();
    let mut language_dto: Option<NewLanguageDto> = None;
    let mut tags_dto: Vec<NewTagDto> = Vec::new();

    for tag in detail.info.tags.iter() {
        let (namespace, raw_tag) = parse_tag(tag);
        if namespace.is_none() {
            continue;
        }
        let (namespace, raw_tag) = (namespace.unwrap(), raw_tag.unwrap());
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
        let raw_tag = raw_tag.to_string();
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
                    lang_code: raw_tag.clone(),
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
    client
        .add_book(dto)
        .map_err(|e| Error::msg(format!("[{}] calibre add failed: {}", gid_token, e)))?;

    Ok(())
}
