use std::{path::PathBuf, vec};

use anyhow::{Result, anyhow};
use axum::{Json, extract::State, http::StatusCode};
use libeh::dto::api::{GIDListItem, GalleryMetadataRequest, GalleryMetadataResponse};
use log::{error, info, warn};
use reqwest::Url;
use serde_json::{Value, json};

use super::{
    ImportRequest,
    utils::{Gallery, calibre::add_to_calibre, extract_cover},
};
use crate::{DownloadManager, g_info, g_warn};

const EH_API_URL: &str = "https://api.e-hentai.org/api.php";

pub async fn handle_import(
    State(manager): State<DownloadManager>,
    Json(request): Json<ImportRequest>,
) -> (StatusCode, Json<Value>) {
    match manager.import_archive(request.url, request.path).await {
        Ok(_) => (StatusCode::OK, Json(json!({}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"msg": format!("导入失败: {}", e)})),
        ),
    }
}

impl DownloadManager {
    pub async fn import_archive(&self, url: String, path: String) -> Result<()> {
        let client = self.client.clone();
        let output = self.output.clone();
        let is_exhentai = self.is_exhentai;
        let calibre_client = self.calibre_client.clone();
        let tag_db = self.tag_db.clone();
        let original_url = url.clone();

        let archive = PathBuf::from(&path);
        if !archive.exists() || !archive.is_file() {
            return Err(anyhow!("Archive not found: {}", path));
        }

        let ext = archive.extension().and_then(|e| e.to_str());
        if ext != Some("zip") && ext != Some("cbz") {
            return Err(anyhow!("File must be a .cbz or .zip archive"));
        }

        tokio::spawn(async move {
            let result: Result<()> = async {
                info!("Starting import: {} (file: {})", url, path);

                let body = GalleryMetadataRequest::new(vec![GIDListItem::from(url)]);
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
                let gid_token = &format!("{}_{}", metadata.gid, metadata.token);
                g_info!(
                    gid_token,
                    "Gallery metadata parsed successfully. Title: {}",
                    metadata.title
                );

                let gallery_dir = format!("{}/{}", output.display(), gid_token);
                let filename = gid_token;
                let output_path = format!("{}/{}.cbz", gallery_dir, filename);

                if PathBuf::from(&output_path).exists() {
                    g_warn!(gid_token, "Archive already exists: {}", output_path);
                } else {
                    tokio::fs::create_dir_all(&gallery_dir).await?;
                    g_info!(gid_token, "Copying archive file to: {}", output_path);
                    tokio::fs::copy(&path, &output_path).await?;
                    g_info!(gid_token, "File copied successfully: {}", output_path);
                }

                let json_path = format!("{}/gallery_metadata.json", gallery_dir);
                g_info!(gid_token, "Saving gallery metadata to JSON: {}", json_path);
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

                g_info!(gid_token, "Adding book to calibre library");
                add_to_calibre(
                    calibre_client,
                    tag_db,
                    is_exhentai,
                    &output_path,
                    &Gallery::Metadata(metadata),
                    gid_token,
                )
                .await?;
                g_info!(gid_token, "Book added to calibre library successfully");

                Ok(())
            }
            .await;
            if let Err(e) = result {
                error!("Import task failed for URL {}: {:?}", original_url, e);
            }
        });

        Ok(())
    }
}
