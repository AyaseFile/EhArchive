use std::{fs::File, path::PathBuf};

use anyhow::{Result, anyhow, ensure};
use axum::{Json, extract::State, http::StatusCode};
use log::{error, info, warn};
use serde_json::{Value, json};

use super::{
    ImportRequest,
    utils::{archive, comic_info},
};
use crate::{DownloadManager, g_info, g_warn};

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
        let archive = PathBuf::from(&path);
        ensure!(archive.is_file(), "Archive does not exist: {path}");
        ensure!(
            archive
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| { matches!(ext.to_ascii_lowercase().as_str(), "zip" | "cbz") }),
            "File must be a .zip or .cbz archive"
        );
        let site = if self.is_exhentai {
            "exhentai.org"
        } else {
            "e-hentai.org"
        };
        let url = if self.is_exhentai {
            url.replace("e-hentai.org", "exhentai.org")
        } else {
            url.replace("exhentai.org", "e-hentai.org")
        };
        {
            let mut tasks = self.active_tasks.lock().await;
            if !tasks.insert(url.clone()) {
                warn!("Import task is already in progress: {url}");
                return Err(anyhow!("Import task is already in progress: {}", url));
            }
        }
        let manager = self.clone();
        tokio::spawn(async move {
            let result: Result<()> = async {
                info!("Starting import: {url} (file: {path})");
                let metadata = manager.get_gallery_metadata(&url).await?;
                let gid_token = format!("{}_{}", metadata.gid, metadata.token);
                g_info!(
                    gid_token,
                    "Gallery metadata parsed successfully. Title: {}",
                    metadata.title
                );
                if let Some(output_path) =
                    archive::find_archive(&manager.output, metadata.gid, &metadata.token)?
                {
                    g_warn!(
                        gid_token,
                        "Archive already exists: {}",
                        output_path.display()
                    );
                    return Ok(());
                }
                let comic_metadata = {
                    let mut tag_db = manager.tag_db.lock().await;
                    comic_info::gallery_to_comic_metadata(site, &metadata, &mut tag_db)?
                };
                let identifier = format!(
                    "{}_{}_{}",
                    metadata.gid,
                    metadata.token,
                    i32::from(manager.is_exhentai)
                );
                let output = manager.output.clone();
                let output_path = tokio::task::spawn_blocking(move || {
                    archive::build_archive(
                        File::open(archive)?,
                        &output,
                        &identifier,
                        &comic_metadata,
                    )
                })
                .await??;
                g_info!(gid_token, "Archive saved successfully: {}", output_path);
                Ok(())
            }
            .await;
            manager.active_tasks.lock().await.remove(&url);
            if let Err(e) = result {
                error!("Import task failed for URL {url}: {e:?}");
            }
        });
        Ok(())
    }
}
