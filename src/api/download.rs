use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use axum::{Json, extract::State, http::StatusCode};
use libeh::dto::gallery::detail::GalleryDetail;
use log::{error, info, warn};
use reqwest::Url;

use super::{
    DownloadRequest, DownloadType,
    utils::{calibre::add_to_calibre, extract_cover},
};
use crate::{DownloadManager, g_info, g_warn};

pub async fn handle_download(
    State(manager): State<DownloadManager>,
    Json(request): Json<DownloadRequest>,
) -> StatusCode {
    let _ = manager
        .download_and_archive(request.url, request.download_type)
        .await;

    StatusCode::NO_CONTENT
}

impl DownloadManager {
    pub async fn download_and_archive(
        &self,
        url: String,
        download_type: DownloadType,
    ) -> Result<()> {
        {
            let tasks = self.active_tasks.lock().await;
            if tasks.contains(&url) {
                warn!("Download job is already in progress: {}", url);
                return Ok(());
            }
        }

        let semaphore = self.semaphore.clone();
        let client = self.client.clone();
        let output = self.output.clone();
        let is_exhentai = self.is_exhentai;
        let calibre_client = self.calibre_client.clone();
        let tag_db = self.tag_db.clone();
        let active_tasks = self.active_tasks.clone();
        let original_url = url.clone();

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            {
                let mut tasks = active_tasks.lock().await;
                tasks.insert(original_url.clone());
            }

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

                let gallery_dir = format!("{}/{}", output.display(), gid_token);
                let filename = format!("{}_{}", detail.info.gid, detail.info.token);
                let output_path = format!("{}/{}.cbz", gallery_dir, filename);

                if PathBuf::from(&output_path).exists() {
                    g_warn!(gid_token, "Archive already exists: {}", output_path);
                } else {
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
                    tokio::fs::create_dir_all(&gallery_dir)
                        .await
                        .with_context(|| {
                            format!(
                                "[{}] Failed to create directory: {}",
                                gid_token, gallery_dir
                            )
                        })?;
                    g_info!(gid_token, "Writing archive to: {}", output_path);
                    tokio::fs::write(&output_path, data)
                        .await
                        .with_context(|| {
                            format!("[{}] Failed to save archive to {}", gid_token, output_path)
                        })?;
                    g_info!(gid_token, "Archive saved successfully: {}", output_path);
                }

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
                let result = extract_cover(&output_path, &gallery_dir)
                    .context("Failed to extract cover image")?;
                if let Some((cover, cover_path)) = result {
                    g_info!(gid_token, "Found cover image: {}", cover);
                    g_info!(gid_token, "Cover image saved to: {}", cover_path);
                } else {
                    g_warn!(gid_token, "No cover image found in archive");
                }

                g_info!(gid_token, "Adding book to calibre library");

                add_to_calibre(
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

                {
                    let mut tasks = active_tasks.lock().await;
                    tasks.remove(&original_url);
                }

                Ok(())
            }
            .await;
            if let Err(e) = result {
                {
                    let mut tasks = active_tasks.lock().await;
                    tasks.remove(&original_url);
                }
                error!("Download job failed: {}", e);
            }
        });

        Ok(())
    }
}
