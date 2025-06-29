use axum::{Json, extract::State};

use super::{
    BookMetadataReplaceRequest, BookMetadataReplaceResponse, MetadataUpdateResponse,
    utils::calibre::{replace_book_metadata, update_metadata},
};
use crate::DownloadManager;

pub async fn handle_metadata_update(
    State(manager): State<DownloadManager>,
) -> Json<MetadataUpdateResponse> {
    let calibre_client = manager.calibre_client.clone();
    let tag_db = manager.tag_db;

    tokio::spawn(async move {
        let _ = update_metadata(calibre_client, tag_db).await;
    });

    Json(MetadataUpdateResponse {
        message: "元数据翻译更新任务已启动".to_string(),
    })
}

pub async fn handle_book_metadata_replace(
    State(manager): State<DownloadManager>,
    Json(request): Json<BookMetadataReplaceRequest>,
) -> Json<BookMetadataReplaceResponse> {
    let is_exhentai = manager.is_exhentai;
    let calibre_client = manager.calibre_client.clone();
    let client = manager.client.clone();
    let tag_db = manager.tag_db;
    let url = request.url;

    tokio::spawn(async move {
        let _ =
            replace_book_metadata(calibre_client, tag_db, client, is_exhentai, url.clone()).await;
    });

    Json(BookMetadataReplaceResponse {
        message: "书籍元数据替换任务已启动".to_string(),
    })
}
