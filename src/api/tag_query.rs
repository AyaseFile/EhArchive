use anyhow::Result;
use axum::{Json, extract::State, http::StatusCode};
use log::{error, info};
use serde_json::{Value, json};

use super::{TagQueryRequest, TagQueryResponse};
use crate::DownloadManager;

pub async fn handle_tag_query(
    State(manager): State<DownloadManager>,
    Json(request): Json<TagQueryRequest>,
) -> (StatusCode, Json<Value>) {
    info!(
        "Querying tag translation for namespace: {}, raw_tag: {}",
        request.namespace, request.raw_tag
    );

    match query_tag_translation(&manager, &request.namespace, &request.raw_tag).await {
        Ok(translated_name) => {
            let response = TagQueryResponse { translated_name };
            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => {
            error!("Failed to query tag translation: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("查询标签翻译失败: {}", e)})),
            )
        }
    }
}

async fn query_tag_translation(
    manager: &DownloadManager,
    namespace: &str,
    raw_tag: &str,
) -> Result<Option<String>> {
    let result = {
        let mut tag_db = manager.tag_db.lock().await;
        tag_db.get_tag_name(namespace, raw_tag)?
    };
    Ok(result)
}
