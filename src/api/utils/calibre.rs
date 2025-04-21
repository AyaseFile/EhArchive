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
use crate::tag_db::db::EhTagDb;
use crate::{g_info, tag_db};

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

    for tag in gallery_tags {
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

pub async fn update_tag_trans(
    calibre_client: Arc<Mutex<CalibreClient>>,
    tag_db: Arc<Mutex<EhTagDb>>,
) -> Result<()> {
    let mut client = calibre_client.lock().await;
    let mut tag_db = tag_db.lock().await;

    let tags_in_tag_db = tag_db.get_all_tags()?;
    let rows_in_tag_db = tags_in_tag_db.get("rows").unwrap();
    let authors_in_calibre = client.get_all_authors().map_err(|e| anyhow!("{}", e))?;
    let publishers_in_calibre = client.get_all_publishers().map_err(|e| anyhow!("{}", e))?;
    let tags_in_calibre = client.get_all_tags().map_err(|e| anyhow!("{}", e))?;

    let mut updated_count = 0;

    for author in authors_in_calibre {
        let namespace = "artist";
        let raw_tag = &author.name;
        let author_name = tags_in_tag_db.get(namespace).unwrap().get(raw_tag);
        if author_name.is_none() {
            continue;
        }
        let author_name = author_name.unwrap();
        if author_name == raw_tag {
            continue;
        }
        client
            .replace_author_with_translation(author.id, author_name)
            .map_err(|e| anyhow!("{}", e))?;
        log::info!(
            "Replaced author {} with translation {}",
            raw_tag,
            author_name
        );
        updated_count += 1;
    }

    if updated_count != 0 {
        log::info!("Updated {} authors in calibre", updated_count);
    } else {
        log::info!("No authors to update in calibre");
    }

    updated_count = 0;

    for publisher in publishers_in_calibre {
        let namespace = "group";
        let raw_tag = &publisher.name;
        let publisher_name = tags_in_tag_db.get(namespace).unwrap().get(raw_tag);
        if publisher_name.is_none() {
            continue;
        }
        let publisher_name = publisher_name.unwrap();
        if publisher_name == raw_tag {
            continue;
        }
        client
            .replace_publisher_with_translation(publisher.id, publisher_name)
            .map_err(|e| anyhow!("{}", e))?;
        log::info!(
            "Replaced publisher {} with translation {}",
            raw_tag,
            publisher_name
        );
        updated_count += 1;
    }

    if updated_count != 0 {
        log::info!("Updated {} publishers in calibre", updated_count);
    } else {
        log::info!("No publishers to update in calibre");
    }

    updated_count = 0;

    for tag in tags_in_calibre {
        let parts: Vec<_> = tag.name.split(':').collect();

        if parts.len() != 2 {
            continue;
        }

        let namespace = parts[0];
        let raw_tag = parts[1];

        if let (Some(tags_map), Some(tag_namespace)) =
            (tags_in_tag_db.get(namespace), rows_in_tag_db.get(namespace))
        {
            let tag_name = tags_map.get(raw_tag);
            if tag_name.is_none() {
                continue;
            }
            let tag_name = tag_name.unwrap();
            if tag_name == raw_tag {
                continue;
            }
            let translation = format!("{}:{}", tag_namespace, tag_name);
            client
                .replace_tag_with_translation(tag.id, &translation)
                .map_err(|e| anyhow!("{}", e))?;
            log::info!("Replaced tag {} with translation {}", tag.name, translation);
            updated_count += 1;
        }
    }

    if updated_count != 0 {
        log::info!("Updated {} tags in calibre", updated_count);
    } else {
        log::info!("No tags to update in calibre");
    }

    Ok(())
}
