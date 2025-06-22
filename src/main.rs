mod api;
mod config;
mod g_log;
mod tag_db;

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use axum::{
    Router,
    routing::{get, post},
};
use libcalibre::{client::CalibreClient, util::get_db_path};
use libeh::{
    client::{auth::EhClientAuth, client::EhClient, config::EhClientConfig},
    dto::site::Site,
};
use tokio::sync::{Mutex, Semaphore};

use api::{
    calibre::handle_tag_update, download::handle_download, import::handle_import,
    tag_query::handle_tag_query, tasks::get_active_tasks,
};
use config::Config;
use tag_db::db::EhTagDb;

#[derive(Clone)]
struct DownloadManager {
    client: EhClient,
    is_exhentai: bool,
    output: PathBuf,
    semaphore: Arc<Semaphore>,
    tag_db: Arc<Mutex<EhTagDb>>,
    calibre_client: Arc<Mutex<CalibreClient>>,
    active_tasks: Arc<Mutex<HashSet<String>>>,
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
        let valid_path = get_db_path(config.library_root());
        let calibre_client = CalibreClient::new(valid_path.unwrap());
        Self {
            client: EhClient::new(eh_client_config),
            is_exhentai: matches!(site, Site::Ex),
            output: config.archive_output().into(),
            semaphore: Arc::new(Semaphore::new(config.limit())),
            tag_db: Arc::new(Mutex::new(tag_db)),
            calibre_client: Arc::new(Mutex::new(calibre_client)),
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
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
        .route("/download", post(handle_download))
        .route("/tasks", get(get_active_tasks))
        .route("/import", post(handle_import))
        .route("/calibre/metadata/update", post(handle_tag_update))
        .route("/tag/query", post(handle_tag_query))
        .with_state(download_manager);

    let addr = format!("0.0.0.0:{}", port);
    log::info!("Server started: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
