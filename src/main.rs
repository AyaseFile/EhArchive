mod api;
mod config;
mod g_log;
mod tag_db;

use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Router,
    routing::{get, post},
};
use libeh::{
    client::{auth::EhClientAuth, client::EhClient, config::EhClientConfig},
    dto::site::Site,
};
use tokio::sync::{Mutex, Semaphore};

use api::{
    download::handle_download, import::handle_import, tag_query::handle_tag_query,
    tasks::get_active_tasks,
};
use config::Config;
use tag_db::db::EhTagDb;

#[derive(Clone)]
struct DownloadManager {
    client: EhClient,
    is_exhentai: bool,
    output: PathBuf,
    metadata_output: Option<PathBuf>,
    semaphore: Arc<Semaphore>,
    tag_db: Arc<Mutex<EhTagDb>>,
    active_tasks: Arc<Mutex<HashSet<String>>>,
    komga: Option<Komga>,
}

#[derive(Clone)]
struct Komga {
    client: reqwest::Client,
    scan_url: String,
    api_key: String,
}

impl DownloadManager {
    fn new(config: Config) -> Self {
        let eh_auth_config = EhClientAuth {
            ipb_member_id: config.ipb_member_id().into(),
            ipb_pass_hash: config.ipb_pass_hash().into(),
            igneous: config.igneous().map(|s| s.into()),
        };
        let site = config.site();
        let eh_client_config = EhClientConfig {
            site,
            proxy: None,
            auth: Some(eh_auth_config),
        };
        let tag_db = EhTagDb::new(config.tag_db_path().into()).unwrap();
        let komga = config.komga().map(|(url, library_id, api_key)| Komga {
            client: reqwest::Client::new(),
            scan_url: format!(
                "{}/api/v1/libraries/{}/scan",
                url.trim_end_matches('/'),
                library_id
            ),
            api_key: api_key.into(),
        });
        Self {
            client: EhClient::new(eh_client_config),
            is_exhentai: matches!(site, Site::Ex),
            output: config.archive_output().into(),
            metadata_output: config.metadata_output().map(PathBuf::from),
            semaphore: Arc::new(Semaphore::new(config.limit())),
            tag_db: Arc::new(Mutex::new(tag_db)),
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
            komga,
        }
    }

    async fn scan_komga(&self) {
        let Some(komga) = &self.komga else {
            return;
        };

        match komga
            .client
            .post(&komga.scan_url)
            .header("X-API-Key", &komga.api_key)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status() == reqwest::StatusCode::ACCEPTED => {
                log::info!("Komga library scan requested successfully")
            }
            Ok(response) => log::warn!(
                "Komga library scan returned unexpected status {} from {}",
                response.status(),
                response.url()
            ),
            Err(e) => log::warn!("Failed to request Komga library scan: {e}"),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .format_target(false)
        .init();

    let config = Config::parse();
    let port = config.port();
    let download_manager = DownloadManager::new(config);

    let app = Router::new()
        .route("/downloads", post(handle_download))
        .route("/tasks", get(get_active_tasks))
        .route("/imports", post(handle_import))
        .route("/tags/query", post(handle_tag_query))
        .with_state(download_manager);

    let addr = format!("0.0.0.0:{port}");
    log::info!("Server started: http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
