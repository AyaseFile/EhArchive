use std::fs::File;

use anyhow::{Result, anyhow, ensure};
use axum::{Json, extract::State, http::StatusCode};
use libeh::dto::{
    api::{GIDListItem, GalleryMetadata, GalleryMetadataRequest, GalleryMetadataResponse},
    gallery::detail::GalleryDetail,
};
use log::{error, info, warn};
use reqwest::Url;
use serde_json::{Value, json};

use super::{
    DownloadRequest, DownloadType, EH_API_URL,
    utils::{archive, comic_info},
};
use crate::{DownloadManager, g_info, g_warn};

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
    pub(super) async fn get_gallery_metadata(&self, url: &str) -> Result<GalleryMetadata> {
        let body = GalleryMetadataRequest::new(vec![GIDListItem::from(url.to_string())]);
        let response: GalleryMetadataResponse = self
            .client
            .post_json(Url::parse(EH_API_URL)?, serde_json::to_string(&body)?)
            .await
            .map_err(|e| anyhow!(e))?;
        let metadata = response
            .gmetadata
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No metadata found"))?;
        if let Some(output) = &self.metadata_output {
            let gid_token = format!("{}_{}", metadata.gid, metadata.token);
            let json_path = output.join(format!("{gid_token}.json"));
            g_info!(
                gid_token,
                "Saving gallery details to JSON: {}",
                json_path.display()
            );
            tokio::fs::create_dir_all(output).await?;
            let json = serde_json::to_string_pretty(&metadata)?;
            tokio::fs::write(json_path, json).await?;
            g_info!(gid_token, "Gallery details saved to JSON successfully");
        }
        Ok(metadata)
    }

    async fn download_and_archive(&self, url: String, download_type: DownloadType) -> Result<()> {
        let url = if self.is_exhentai {
            url.replace("e-hentai.org", "exhentai.org")
        } else {
            url.replace("exhentai.org", "e-hentai.org")
        };
        {
            let mut tasks = self.active_tasks.lock().await;
            if !tasks.insert(url.clone()) {
                warn!("Download job is already in progress: {url}");
                return Err(anyhow!("Download job is already in progress: {}", url));
            }
        }
        let manager = self.clone();
        tokio::spawn(async move {
            let _permit = manager.semaphore.acquire().await.unwrap();
            let result = manager.run_download(&url, download_type).await;
            manager.active_tasks.lock().await.remove(&url);
            if let Err(e) = result {
                error!("Download job failed for URL {url}: {e:?}");
            }
        });
        Ok(())
    }

    async fn run_download(&self, url: &str, download_type: DownloadType) -> Result<()> {
        info!("Starting download: {url} (type: {download_type})");
        let site = if self.is_exhentai {
            "exhentai.org"
        } else {
            "e-hentai.org"
        };
        let html = self
            .client
            .get_html(Url::parse(url)?)
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
        let metadata = self.get_gallery_metadata(url).await?;
        let gid_token = format!("{}_{}", metadata.gid, metadata.token);
        g_info!(
            gid_token,
            "Gallery metadata parsed successfully. Title: {}",
            metadata.title
        );
        ensure!(
            detail.info.gid == metadata.gid && detail.info.token == metadata.token,
            "Gallery detail and API metadata do not identify the same gallery"
        );
        if let Some(output_path) =
            archive::find_archive(&self.output, metadata.gid, &metadata.token)?
        {
            g_warn!(
                gid_token,
                "Archive already exists: {}",
                output_path.display()
            );
            return Ok(());
        }
        let comic_metadata = {
            let mut tag_db = self.tag_db.lock().await;
            comic_info::gallery_to_comic_metadata(site, &metadata, &mut tag_db)?
        };
        let identifier = format!(
            "{}_{}_{}",
            metadata.gid,
            metadata.token,
            i32::from(self.is_exhentai)
        );
        tokio::fs::create_dir_all(&self.output).await?;
        let (download_file, download_path) = tempfile::Builder::new()
            .prefix(&format!(".{identifier}."))
            .suffix(".zip.partial")
            .tempfile_in(&self.output)?
            .into_parts();
        let mut download_file = tokio::fs::File::from_std(download_file);
        let archive_size = detail
            .download_archive(
                &self.client,
                matches!(&download_type, DownloadType::Original),
                &mut download_file,
            )
            .await
            .map_err(|e| anyhow!(e))?;
        drop(download_file);
        g_info!(
            gid_token,
            "Archive download completed successfully ({} bytes)",
            archive_size
        );
        let output = self.output.clone();
        let output_path = output.join(format!("{identifier}.cbz"));
        g_info!(gid_token, "Writing archive to: {}", output_path.display());
        let output_path = tokio::task::spawn_blocking(move || {
            archive::build_archive(
                File::open(&download_path)?,
                &output,
                &identifier,
                &comic_metadata,
            )
        })
        .await??;
        g_info!(gid_token, "Archive saved successfully: {}", output_path);
        self.scan_komga().await;
        Ok(())
    }
}
