use axum::{Json, extract::State};

use super::{TagUpdateResponse, utils::calibre::update_tag_trans};
use crate::DownloadManager;

pub async fn handle_tag_update(State(manager): State<DownloadManager>) -> Json<TagUpdateResponse> {
    match update_tag_trans(manager.calibre_client.clone(), manager.tag_db.clone()).await {
        Ok(_) => Json(TagUpdateResponse {
            success: true,
            message: "元数据翻译更新成功".to_string(),
        }),
        Err(e) => {
            log::error!("元数据翻译更新失败: {}", e);
            Json(TagUpdateResponse {
                success: false,
                message: format!("元数据翻译更新失败: {}", e),
            })
        }
    }
}
