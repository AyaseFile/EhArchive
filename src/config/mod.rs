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
    #[clap(long, env = "CALIBRE_LIBRARY_ROOT")]
    library_root: String,
    #[clap(long, env = "TAG_DB_ROOT")]
    tag_db_root: String,

    #[clap(long, env = "LIMIT", default_value = "5")]
    limit: usize,
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

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn archive_output(&self) -> &str {
        &self.archive_output
    }

    pub fn library_root(&self) -> &str {
        &self.library_root
    }

    pub fn tag_db_path(&self) -> &str {
        &self.tag_db_root
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}
