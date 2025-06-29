use axum::{Json, extract::State};

use super::{
    BookMetadataReplaceRequest, BookMetadataReplaceResponse, MetadataUpdateResponse,
    utils::calibre::{replace_book_metadata, update_metadata},
};
use crate::DownloadManager;

pub async fn handle_metadata_update(
    State(manager): State<DownloadManager>,
) -> Json<MetadataUpdateResponse> {
    match update_metadata(manager.calibre_client.clone(), manager.tag_db.clone()).await {
        Ok(_) => Json(MetadataUpdateResponse {
            success: true,
            message: "元数据翻译更新成功".to_string(),
        }),
        Err(e) => {
            log::error!("元数据翻译更新失败: {e}");
            Json(MetadataUpdateResponse {
                success: false,
                message: format!("元数据翻译更新失败: {e}"),
            })
        }
    }
}

pub async fn handle_book_metadata_replace(
    State(manager): State<DownloadManager>,
    Json(request): Json<BookMetadataReplaceRequest>,
) -> Json<BookMetadataReplaceResponse> {
    match replace_book_metadata(
        manager.calibre_client.clone(),
        manager.tag_db.clone(),
        manager.client.clone(),
        manager.is_exhentai,
        request.url,
    )
    .await
    {
        Ok(_) => Json(BookMetadataReplaceResponse {
            success: true,
            message: "书籍元数据替换成功".to_string(),
        }),
        Err(e) => {
            log::error!("书籍元数据替换失败: {e}");
            Json(BookMetadataReplaceResponse {
                success: false,
                message: format!("书籍元数据替换失败: {e}"),
            })
        }
    }
}
