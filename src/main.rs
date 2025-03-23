mod api;
mod config;
mod g_log;
mod tag_db;

use std::{fs::File, io, path::PathBuf, sync::Arc};

use anyhow::{Context, Error, Result};
use api::{DownloadRequest, DownloadType};
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use config::Config;
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
    util::get_db_path,
};
use libeh::{
    client::{auth::EhClientAuth, client::EhClient, config::EhClientConfig},
    dto::{
        gallery::{category::Category, detail::GalleryDetail},
        keyword::Keyword,
        site::Site,
    },
};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Url;
use tag_db::db::EhTagDb;
use tokio::sync::{Mutex, Semaphore};
use zip::ZipArchive;

#[derive(Clone)]
struct DownloadManager {
    client: EhClient,
    is_exhentai: bool,
    output: PathBuf,
    semaphore: Arc<Semaphore>,
    tag_db: Arc<Mutex<EhTagDb>>,
    calibre_client: Arc<Mutex<CalibreClient>>,
}

static TITLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:\([^\(\)]+\))?\s*(?:\[[^\[\]]+\])?\s*([^\[\]\(\)]+)").unwrap());

impl DownloadManager {
    fn new(config: Config) -> Self {
        let eh_auth_config = EhClientAuth {
            ipb_member_id: config.ipb_member_id().into(),
            ipb_pass_hash: config.ipb_pass_hash().into(),
            igneous: config.igneous().map(|s| s.into()),
        };
        let site = config.site();
        let eh_client_config = EhClientConfig {
            site,
            proxy: None,
            auth: Some(eh_auth_config),
        };
        let tag_db = EhTagDb::new(config.tag_db_path().into()).unwrap();
        let valid_path = get_db_path(config.library_root());
        let calibre_client = CalibreClient::new(valid_path.unwrap());
        Self {
            client: EhClient::new(eh_client_config),
            is_exhentai: matches!(site, Site::Ex),
            output: config.archive_output().into(),
            semaphore: Arc::new(Semaphore::new(config.limit())),
            tag_db: Arc::new(Mutex::new(tag_db)),
            calibre_client: Arc::new(Mutex::new(calibre_client)),
        }
    }

    async fn download_and_archive(&self, url: String, download_type: DownloadType) -> Result<()> {
        let semaphore = self.semaphore.clone();
        let client = self.client.clone();
        let output = self.output.clone();
        let is_exhentai = self.is_exhentai;
        let calibre_client = self.calibre_client.clone();
        let tag_db = self.tag_db.clone();

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let result: Result<()> = async {
                info!("Starting download: {} (type: {})", url, download_type);

                let url =
                    Url::parse(&url).with_context(|| format!("URL parsing failed: {}", url))?;
                let html = client
                    .get_html(url.clone())
                    .await
                    .map_err(Error::msg)
                    .context("Failed to fetch HTML")?;
                let detail = GalleryDetail::parse(html)
                    .map_err(Error::msg)
                    .context(format!("Failed to parse gallery details: {}", url))?;

                let gid_token = format!("{}_{}", detail.info.gid, detail.info.token);

                g_info!(
                    gid_token,
                    "Gallery details parsed successfully. Title: {}, Size: {}",
                    detail.info.title,
                    detail.size
                );
                let is_original = match download_type {
                    DownloadType::Original => true,
                    DownloadType::Resample => false,
                };

                let data = detail
                    .download_archive(&client, is_original)
                    .await
                    .map_err(Error::msg)
                    .context(format!("[{}] Archive download failed", gid_token))?;
                g_info!(
                    gid_token,
                    "Archive download completed successfully ({} bytes)",
                    data.len()
                );

                let gallery_dir = format!("{}/{}", output.display(), gid_token);
                tokio::fs::create_dir_all(&gallery_dir)
                    .await
                    .with_context(|| {
                        format!(
                            "[{}] Failed to create directory: {}",
                            gid_token, gallery_dir
                        )
                    })?;

                let filename = format!(
                    "{}_{}_{}",
                    detail.info.gid,
                    detail.info.token,
                    if is_exhentai { 1 } else { 0 }
                );
                let output_path = format!("{}/{}.cbz", gallery_dir, filename);
                g_info!(gid_token, "Writing archive to: {}", output_path);

                tokio::fs::write(&output_path, data)
                    .await
                    .with_context(|| {
                        format!("[{}] Failed to save archive to {}", gid_token, output_path)
                    })?;
                g_info!(gid_token, "Archive saved successfully: {}", output_path);

                let json_path = format!("{}/gallery_detail.json", gallery_dir);
                g_info!(gid_token, "Saving gallery details to JSON: {}", json_path);
                let json = serde_json::to_string_pretty(&detail).with_context(|| {
                    format!(
                        "[{}] Failed to serialize gallery details to JSON",
                        gid_token
                    )
                })?;
                tokio::fs::write(&json_path, json).await.with_context(|| {
                    format!(
                        "[{}] Failed to save JSON metadata to {}",
                        gid_token, json_path
                    )
                })?;
                g_info!(gid_token, "Gallery details saved to JSON successfully");

                g_info!(gid_token, "Extracting cover image");
                let result = Self::extract_cover(&output_path, &gallery_dir)
                    .context("Failed to extract cover image")?;
                if let Some((cover, cover_path)) = result {
                    g_info!(gid_token, "Found cover image: {}", cover);
                    g_info!(gid_token, "Cover image saved to: {}", cover_path);
                } else {
                    g_warn!(gid_token, "No cover image found in archive");
                }

                g_info!(gid_token, "Adding book to calibre library");

                Self::add_to_calibre(
                    calibre_client,
                    tag_db,
                    is_exhentai,
                    output_path,
                    detail,
                    gid_token.clone(),
                )
                .await
                .context(format!(
                    "[{}] Failed to add book to calibre library",
                    gid_token
                ))?;

                g_info!(gid_token, "Book added to calibre library successfully");

                Ok(())
            }
            .await;
            if let Err(e) = result {
                error!("Download job failed: {}", e);
            }
        });

        Ok(())
    }

    async fn add_to_calibre(
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

        let category = Self::parse_category(&detail.info.category);
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
            let (namespace, raw_tag) = Self::parse_tag(tag);
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

    fn parse_category(category: &Category) -> Option<&str> {
        match category {
            Category::None => None,
            Category::Misc => Some("misc"),
            Category::Doujinshi => Some("doujinshi"),
            Category::Manga => Some("manga"),
            Category::ArtistCG => Some("artistcg"),
            Category::GameCG => Some("gamecg"),
            Category::ImageSet => Some("imageset"),
            Category::Cosplay => Some("cosplay"),
            Category::NonH => Some("non-h"),
            Category::Western => Some("western"),
            Category::All => None,
            Category::Private => Some("private"),
            Category::Unknown => None,
        }
    }

    fn parse_tag(tag: &Keyword) -> (Option<&str>, Option<&str>) {
        match tag {
            Keyword::Normal(_) => (None, None),
            Keyword::Language(k) => (Some("language"), Some(k)),
            Keyword::Parody(k) => (Some("parody"), Some(k)),
            Keyword::Character(k) => (Some("character"), Some(k)),
            Keyword::Artist(k) => (Some("artist"), Some(k)),
            Keyword::Cosplayer(k) => (Some("cosplayer"), Some(k)),
            Keyword::Group(k) => (Some("group"), Some(k)),
            Keyword::Female(k) => (Some("female"), Some(k)),
            Keyword::Male(k) => (Some("male"), Some(k)),
            Keyword::Mixed(k) => (Some("mixed"), Some(k)),
            Keyword::Other(k) => (Some("other"), Some(k)),
            Keyword::Reclass(k) => (Some("reclass"), Some(k)),
            Keyword::Temp(_) => (None, None),
            Keyword::Uploader(k) => (Some("uploader"), Some(k)),
        }
    }

    pub fn extract_cover(cbz_path: &str, output_dir: &str) -> Result<Option<(String, String)>> {
        let file = File::open(cbz_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let path = file.mangled_name();

            if let Some(ext) = path.extension() {
                if let Some(ext) = ext.to_str() {
                    let ext = ext.to_lowercase();
                    if ext == "jpg" || ext == "jpeg" || ext == "png" {
                        let output_path = format!("{}/cover.{}", output_dir, ext);
                        let mut output_file = File::create(&output_path)?;
                        io::copy(&mut file, &mut output_file)?;
                        return Ok(Some((file.name().to_string(), output_path)));
                    }
                }
            }
        }
        Ok(None)
    }
}

async fn handle_download(
    State(manager): State<DownloadManager>,
    Json(request): Json<DownloadRequest>,
) -> StatusCode {
    let _ = manager
        .download_and_archive(request.url, request.download_type)
        .await;

    StatusCode::NO_CONTENT
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .format_target(false)
        .init();

    let config = Config::parse();
    let port = config.port();
    let download_manager = DownloadManager::new(config);

    let app = Router::new()
        .route("/download", post(handle_download))
        .with_state(download_manager);

    let addr = format!("0.0.0.0:{}", port);
    info!("Server started: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
