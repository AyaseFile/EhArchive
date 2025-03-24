use axum::{Json, extract::State};

use super::ActiveTasksResponse;
use crate::DownloadManager;

pub async fn get_active_tasks(State(manager): State<DownloadManager>) -> Json<ActiveTasksResponse> {
    let active_tasks = manager.active_tasks.lock().await;
    let tasks = active_tasks.iter().cloned().collect();

    Json(ActiveTasksResponse {
        count: active_tasks.len(),
        tasks,
    })
}
