use clap::Parser;
use libeh::dto::site::Site;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(env = "EH_AUTH_ID")]
    ipb_member_id: String,
    #[clap(env = "EH_AUTH_HASH")]
    ipb_pass_hash: String,
    #[clap(env = "EH_AUTH_IGNEOUS")]
    igneous: Option<String>,
    #[clap(env = "EH_SITE", default_value = "e-hentai.org")]
    site: String,

    #[clap(long, env = "PORT", default_value = "3000")]
    port: u16,
    #[clap(long, env = "ARCHIVE_OUTPUT")]
    archive_output: String,
    #[clap(long, env = "METADATA_OUTPUT")]
    metadata_output: Option<String>,
    #[clap(long, env = "TAG_DB_ROOT")]
    tag_db_root: String,

    #[clap(long, env = "LIMIT", default_value = "5")]
    limit: usize,

    #[clap(
        long,
        env = "KOMGA_URL",
        requires_all = ["komga_library_id", "komga_api_key"]
    )]
    komga_url: Option<String>,
    #[clap(
        long,
        env = "KOMGA_LIBRARY_ID",
        requires_all = ["komga_url", "komga_api_key"]
    )]
    komga_library_id: Option<String>,
    #[clap(
        long,
        env = "KOMGA_API_KEY",
        requires_all = ["komga_url", "komga_library_id"]
    )]
    komga_api_key: Option<String>,
}

impl Config {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    pub fn ipb_member_id(&self) -> &str {
        &self.ipb_member_id
    }

    pub fn ipb_pass_hash(&self) -> &str {
        &self.ipb_pass_hash
    }

    pub fn igneous(&self) -> Option<&str> {
        self.igneous.as_deref()
    }

    pub fn site(&self) -> Site {
        Site::from(self.site.clone())
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn archive_output(&self) -> &str {
        &self.archive_output
    }

    pub fn metadata_output(&self) -> Option<&str> {
        self.metadata_output.as_deref()
    }

    pub fn tag_db_path(&self) -> &str {
        &self.tag_db_root
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }

    pub fn komga(&self) -> Option<(&str, &str, &str)> {
        match (
            self.komga_url.as_deref(),
            self.komga_library_id.as_deref(),
            self.komga_api_key.as_deref(),
        ) {
            (Some(url), Some(library_id), Some(api_key)) => Some((url, library_id, api_key)),
            _ => None,
        }
    }
}
