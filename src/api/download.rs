use std::path::PathBuf;

use anyhow::{Result, anyhow};
use axum::{Json, extract::State, http::StatusCode};
use libeh::dto::{
    api::{GIDListItem, GalleryMetadataRequest, GalleryMetadataResponse},
    gallery::detail::GalleryDetail,
};
use log::{error, info, warn};
use reqwest::Url;
use serde_json::{Value, json};

use super::{
    DownloadRequest, DownloadType,
    utils::{calibre::add_to_calibre, extract_cover},
};
use crate::{DownloadManager, api::EH_API_URL, g_info, g_warn};

pub async fn handle_download(
    State(manager): State<DownloadManager>,
    Json(request): Json<DownloadRequest>,
) -> (StatusCode, Json<Value>) {
    match manager
        .download_and_archive(request.url, request.download_type)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"msg": format!("启动下载任务失败: {}", e)})),
        ),
    }
}

impl DownloadManager {
    async fn download_and_archive(&self, url: String, download_type: DownloadType) -> Result<()> {
        let is_exhentai = self.is_exhentai;

        let url = if is_exhentai {
            url.replace("e-hentai.org", "exhentai.org")
        } else {
            url.replace("exhentai.org", "e-hentai.org")
        };

        {
            let tasks = self.active_tasks.lock().await;
            if tasks.contains(&url) {
                warn!("Download job is already in progress: {url}");
                return Err(anyhow!("Download job is already in progress: {}", url));
            }
        }

        let semaphore = self.semaphore.clone();
        let client = self.client.clone();
        let output = self.output.clone();
        let calibre_client = self.calibre_client.clone();
        let tag_db = self.tag_db.clone();
        let active_tasks = self.active_tasks.clone();

        tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            {
                let mut tasks = active_tasks.lock().await;
                tasks.insert(url.clone());
            }

            let result: Result<()> = async {
                info!("Starting download: {url} (type: {download_type})");

                let html = client
                    .get_html(Url::parse(&url)?)
                    .await
                    .map_err(|e| anyhow!(e))?;
                let detail = GalleryDetail::parse(html).map_err(|e| anyhow!(e))?;
                let gid_token = format!("{}_{}", detail.info.gid, detail.info.token);
                g_info!(
                    gid_token,
                    "Gallery details parsed successfully. Title: {}, Size: {}",
                    detail.info.title,
                    detail.size
                );

                let body = GalleryMetadataRequest::new(vec![GIDListItem::from(url.clone())]);
                let body = serde_json::to_string(&body).unwrap();
                let api_url = Url::parse(EH_API_URL).unwrap();
                let response: GalleryMetadataResponse = client
                    .post_json(api_url, body)
                    .await
                    .map_err(|e| anyhow!(e))?;
                let metadata = response
                    .gmetadata
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("No metadata found"))?;
                g_info!(
                    gid_token,
                    "Gallery metadata parsed successfully. Title: {}",
                    metadata.title
                );

                let gallery_dir = format!("{}/{}", output.display(), gid_token);
                let filename = &gid_token;
                let output_path = format!("{gallery_dir}/{filename}.cbz");

                if PathBuf::from(&output_path).exists() {
                    g_warn!(gid_token, "Archive already exists: {}", output_path);
                } else {
                    let is_original = match download_type {
                        DownloadType::Original => true,
                        DownloadType::Resample => false,
                    };
                    let data = detail
                        .download_archive(&client, is_original)
                        .await
                        .map_err(|e| anyhow!(e))?;
                    g_info!(
                        gid_token,
                        "Archive download completed successfully ({} bytes)",
                        data.len()
                    );
                    tokio::fs::create_dir_all(&gallery_dir).await?;
                    g_info!(gid_token, "Writing archive to: {}", output_path);
                    tokio::fs::write(&output_path, data).await?;
                    g_info!(gid_token, "Archive saved successfully: {}", output_path);
                }

                let json_path = format!("{gallery_dir}/gallery_detail.json");
                g_info!(gid_token, "Saving gallery details to JSON: {}", json_path);
                let json = serde_json::to_string_pretty(&metadata)?;
                tokio::fs::write(&json_path, json).await?;
                g_info!(gid_token, "Gallery details saved to JSON successfully");

                g_info!(gid_token, "Extracting cover image");
                let result = extract_cover(&output_path, &gallery_dir)?;
                if let Some((cover, cover_path)) = result {
                    g_info!(gid_token, "Found cover image: {}", cover);
                    g_info!(gid_token, "Cover image saved to: {}", cover_path);
                } else {
                    g_warn!(gid_token, "No cover image found in archive");
                }

                add_to_calibre(
                    calibre_client,
                    tag_db,
                    is_exhentai,
                    output_path,
                    metadata,
                    &gid_token,
                )
                .await?;
                g_info!(gid_token, "Book added to calibre library successfully");

                {
                    let mut tasks = active_tasks.lock().await;
                    tasks.remove(&url);
                }

                Ok(())
            }
            .await;
            if let Err(e) = result {
                {
                    let mut tasks = active_tasks.lock().await;
                    tasks.remove(&url);
                }
                error!("Download job failed for URL {url}: {e:?}");
            }
        });

        Ok(())
    }
}
