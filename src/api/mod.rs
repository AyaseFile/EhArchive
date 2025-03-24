pub mod download;
pub mod import;
pub mod tasks;
mod utils;

use std::fmt::{self, Display};

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadType {
    Original,
    Resample,
}

impl Display for DownloadType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub url: String,
    pub download_type: DownloadType,
}

#[derive(Debug, Serialize)]
pub struct ActiveTasksResponse {
    pub count: usize,
    pub tasks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub url: String,
    pub path: String,
}
