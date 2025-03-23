use std::fmt::{self, Display};

use serde::Deserialize;

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
