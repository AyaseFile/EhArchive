pub mod calibre;
pub mod download;
pub mod import;
pub mod tag_query;
pub mod tasks;
mod utils;

use std::fmt::{self, Display};

use serde::Deserialize;
use serde::Serialize;

pub const EH_API_URL: &str = "https://api.e-hentai.org/api.php";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadType {
    Original,
    Resample,
}

impl Display for DownloadType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
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

#[derive(Serialize)]
pub struct MetadataUpdateResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TagQueryRequest {
    pub namespace: String,
    pub raw_tag: String,
}

#[derive(Debug, Serialize)]
pub struct TagQueryResponse {
    pub translated_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BookMetadataReplaceRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct BookMetadataReplaceResponse {
    pub message: String,
}
