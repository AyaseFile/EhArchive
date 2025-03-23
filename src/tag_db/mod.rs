pub mod db;
pub mod models;
pub mod schema;

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36 Edg/121.0.0.0";
const DB_FILENAME: &str = "eh_tag.db";
const CHUNK_SIZE: usize = 500;
const REPO: &str = "EhTagTranslation/Database";
const NAMESPACES: &[&str] = &[
    "artist",
    "character",
    "cosplayer",
    "female",
    "group",
    "language",
    "male",
    "mixed",
    "other",
    "parody",
    "reclass",
    "rows",
];

static ALPHA_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z]").unwrap());

#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct EhTagJson {
    data: Vec<EhTagNamespace>,
}

#[derive(Debug, Deserialize)]
struct EhTagNamespace {
    namespace: String,
    data: HashMap<String, EhTagData>,
}

#[derive(Debug, Deserialize)]
struct EhTagData {
    name: String,
    intro: String,
    links: String,
}

#[derive(Debug)]
struct TagOperation {
    raw: String,
    name: String,
    intro: String,
    links: String,
    operation: TagAction,
}

#[derive(Debug)]
enum TagAction {
    Insert,
    Update,
    Skip,
}
