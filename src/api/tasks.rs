use axum::{Json, extract::State};

use super::ActiveTasksResponse;
use crate::DownloadManager;

pub async fn get_active_tasks(State(manager): State<DownloadManager>) -> Json<ActiveTasksResponse> {
    let tasks: Vec<String> = {
        let active_tasks = manager.active_tasks.lock().await;
        active_tasks.iter().cloned().collect()
    };

    Json(ActiveTasksResponse {
        count: tasks.len(),
        tasks,
    })
}
