use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use diesel::connection::Connection as DieselConnection;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;
use diesel::sqlite::SqliteConnection;
use diesel_dynamic_schema::table;
use log::{debug, info};

use super::models::Metadata;
use super::schema::metadata::dsl as metadata_dsl;
use super::{
    ALPHA_REGEX, CHUNK_SIZE, DB_FILENAME, EhTagJson, GitHubTag, NAMESPACES, REPO, TagAction,
    TagOperation, USER_AGENT,
};

pub struct EhTagDb {
    conn: SqliteConnection,
}

impl EhTagDb {
    pub fn new(path: String) -> Result<Self> {
        let db_path = PathBuf::from(path).join(DB_FILENAME);
        let db_path_str = db_path.to_string_lossy();

        info!("Opening or creating database at: {}", db_path_str);

        let conn = SqliteConnection::establish(&db_path_str)?;

        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&mut self) -> Result<()> {
        let latest_tag = Self::get_latest_github_tag()?;
        info!("Latest GitHub tag: {}", latest_tag);

        let stored_version = self.get_stored_version()?;

        match stored_version {
            Some(version) if version == latest_tag => {
                info!("Database is already at the latest version: {}", version);
                return Ok(());
            }
            Some(version) => {
                info!(
                    "Updating database from version {} to {}",
                    version, latest_tag
                );
            }
            None => {
                info!(
                    "No version found in database, creating new database with version {}",
                    latest_tag
                );
            }
        }

        for namespace in NAMESPACES {
            let table_name = if *namespace == "group" {
                "groups".to_string()
            } else {
                namespace.to_string()
            };
            self.ensure_table_exists(&table_name)?;
        }

        info!("Fetching JSON data from GitHub");
        let json_data = Self::fetch_json_from_github(&latest_tag)?;
        info!("Successfully fetched JSON data");

        for namespace in NAMESPACES {
            info!("Processing namespace: {}", namespace);

            let tag_list = Self::read_tags_from_json(&json_data, namespace)?;

            info!(
                "Updating database with {} tags for namespace {}",
                tag_list.len(),
                namespace
            );

            self.update_namespace(namespace, &tag_list)?;
        }

        info!("Updating stored version to {}", latest_tag);
        self.update_stored_version(&latest_tag)?;
        Ok(())
    }

    pub fn get_tag_name(&mut self, namespace: &str, raw_tag: &str) -> Result<Option<String>> {
        let table_name = if namespace == "group" {
            "groups"
        } else {
            namespace
        };

        let dyn_table = table(table_name);
        let raw_col = dyn_table.column::<Text, _>("raw");
        let name_col = dyn_table.column::<Text, _>("name");

        let result = dyn_table
            .select(name_col)
            .filter(raw_col.eq(raw_tag))
            .first::<String>(&mut self.conn)
            .optional()?;

        Ok(result)
    }

    fn get_existing_tags(
        &mut self,
        namespace: &str,
        raw_values: &[String],
    ) -> Result<HashMap<String, (String, String, String)>> {
        let mut existing_records = HashMap::new();

        if raw_values.is_empty() {
            return Ok(existing_records);
        }

        let table_name = if namespace == "group" {
            "groups"
        } else {
            namespace
        };

        let dyn_table = table(table_name);
        let raw_col = dyn_table.column::<Text, _>("raw");
        let name_col = dyn_table.column::<Text, _>("name");
        let intro_col = dyn_table.column::<Text, _>("intro");
        let links_col = dyn_table.column::<Text, _>("links");

        for chunk in raw_values.chunks(CHUNK_SIZE) {
            for raw_val in chunk {
                let result = dyn_table
                    .select((raw_col, name_col, intro_col, links_col))
                    .filter(raw_col.eq(raw_val))
                    .first::<(String, String, String, String)>(&mut self.conn)
                    .optional()?;

                if let Some((raw, name, intro, links)) = result {
                    existing_records.insert(raw, (name, intro, links));
                }
            }
        }

        Ok(existing_records)
    }

    fn update_namespace(&mut self, namespace: &str, tag_list: &[Vec<String>]) -> Result<()> {
        info!("Updating namespace: {}", namespace);

        let mut raw_values = Vec::new();
        for tags in tag_list {
            if !tags.is_empty() {
                raw_values.push(tags[0].trim().to_string());
            }
        }

        info!("Getting existing tags for namespace {}", namespace);
        let existing_records = self.get_existing_tags(namespace, &raw_values)?;
        info!("Found {} existing records", existing_records.len());

        info!("Determining operations needed");
        let operations = Self::determine_operations(tag_list, &existing_records);

        info!("Executing database operations");
        let (inserts, updates, skips) = self.execute_operations(namespace, operations)?;

        info!(
            "Inserts: {}, Updates: {}, Skips: {}",
            inserts, updates, skips
        );

        Ok(())
    }

    fn execute_operations(
        &mut self,
        namespace: &str,
        operations: Vec<TagOperation>,
    ) -> Result<(usize, usize, usize)> {
        let mut inserts = 0;
        let mut updates = 0;
        let mut skips = 0;

        let table_name = if namespace == "group" {
            "groups"
        } else {
            namespace
        };

        let insert_ops: Vec<_> = operations
            .iter()
            .filter(|op| matches!(op.operation, TagAction::Insert))
            .collect();

        let update_ops: Vec<_> = operations
            .iter()
            .filter(|op| matches!(op.operation, TagAction::Update))
            .collect();

        let skips_count = operations.len() - insert_ops.len() - update_ops.len();

        self.conn
            .transaction::<_, diesel::result::Error, _>(|conn| {
                for op in &insert_ops {
                    let insert_sql = format!(
                        "INSERT INTO {} (raw, name, intro, links) VALUES (?, ?, ?, ?)",
                        table_name
                    );

                    sql_query(insert_sql)
                        .bind::<Text, _>(&op.raw)
                        .bind::<Text, _>(&op.name)
                        .bind::<Text, _>(&op.intro)
                        .bind::<Text, _>(&op.links)
                        .execute(conn)?;
                }

                for op in &update_ops {
                    let update_sql = format!(
                        "UPDATE {} SET name = ?, intro = ?, links = ? WHERE raw = ?",
                        table_name
                    );

                    sql_query(update_sql)
                        .bind::<Text, _>(&op.name)
                        .bind::<Text, _>(&op.intro)
                        .bind::<Text, _>(&op.links)
                        .bind::<Text, _>(&op.raw)
                        .execute(conn)?;
                }

                inserts = insert_ops.len();
                updates = update_ops.len();
                skips = skips_count;

                Ok(())
            })?;

        Ok((inserts, updates, skips))
    }

    fn ensure_table_exists(&mut self, table_name: &str) -> Result<()> {
        let create_table_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY AUTOINCREMENT, 
                raw TEXT NOT NULL, 
                name TEXT NOT NULL, 
                intro TEXT, 
                links TEXT, 
                UNIQUE (raw)
            )",
            table_name
        );

        sql_query(create_table_sql).execute(&mut self.conn)?;

        Ok(())
    }

    fn ensure_metadata_table_exists(&mut self) -> Result<()> {
        sql_query(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        )
        .execute(&mut self.conn)?;

        Ok(())
    }

    fn get_latest_github_tag() -> Result<String> {
        let url = format!("https://api.github.com/repos/{}/tags", REPO);
        info!("Fetching latest tag from: {}", url);

        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .build()?;

        let response = client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API returned status {}", response.status()));
        }

        let tags: Vec<GitHubTag> = response.json()?;

        if tags.is_empty() {
            return Err(anyhow!("No tags found for repository {}", REPO));
        }

        Ok(tags.first().unwrap().name.clone())
    }

    fn get_stored_version(&mut self) -> Result<Option<String>> {
        self.ensure_metadata_table_exists()?;

        let result = metadata_dsl::metadata
            .filter(metadata_dsl::key.eq("github_tag"))
            .select(metadata_dsl::value)
            .first::<String>(&mut self.conn)
            .optional()?;

        Ok(result)
    }

    fn update_stored_version(&mut self, version: &str) -> Result<()> {
        let version_record = Metadata {
            key: "github_tag".to_string(),
            value: version.to_string(),
        };

        diesel::insert_into(metadata_dsl::metadata)
            .values(&version_record)
            .on_conflict(metadata_dsl::key)
            .do_update()
            .set(metadata_dsl::value.eq(&version_record.value))
            .execute(&mut self.conn)?;

        Ok(())
    }

    fn fetch_json_from_github(tag: &str) -> Result<EhTagJson> {
        let url = format!(
            "https://github.com/{}/releases/download/{}/db.text.json",
            REPO, tag
        );
        info!("Fetching JSON from: {}", url);

        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .build()?;

        let response = client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "GitHub returned status {} for {}",
                response.status(),
                url
            ));
        }

        let json_data: EhTagJson = response.json()?;

        Ok(json_data)
    }

    fn read_tags_from_json(json_data: &EhTagJson, namespace: &str) -> Result<Vec<Vec<String>>> {
        let namespace_data = json_data
            .data
            .iter()
            .find(|ns| ns.namespace == namespace)
            .ok_or_else(|| anyhow!("Namespace '{}' not found in JSON data", namespace))?;

        let mut tag_entries = Vec::new();

        for (raw_tag, tag_data) in &namespace_data.data {
            if Self::check(raw_tag) {
                let tag_parts = vec![
                    raw_tag.clone(),
                    tag_data.name.clone(),
                    tag_data.intro.clone(),
                    tag_data.links.clone(),
                ];
                tag_entries.push(tag_parts);
            }
        }

        debug!(
            "Read {} tag entries for namespace {}",
            tag_entries.len(),
            namespace
        );

        Ok(tag_entries)
    }

    fn check(s: &str) -> bool {
        ALPHA_REGEX.is_match(s)
    }

    fn determine_operations(
        tag_list: &[Vec<String>],
        existing_records: &HashMap<String, (String, String, String)>,
    ) -> Vec<TagOperation> {
        let mut operations = Vec::with_capacity(tag_list.len());

        for tags in tag_list {
            let raw_value = tags.first().cloned().unwrap_or_default().trim().to_string();
            let name = tags.get(1).cloned().unwrap_or_default().trim().to_string();
            let intro = tags.get(2).cloned().unwrap_or_default().trim().to_string();
            let links = tags.get(3).cloned().unwrap_or_default().trim().to_string();

            if let Some((db_name, db_intro, db_links)) = existing_records.get(&raw_value) {
                if &name != db_name || &intro != db_intro || &links != db_links {
                    operations.push(TagOperation {
                        raw: raw_value,
                        name,
                        intro,
                        links,
                        operation: TagAction::Update,
                    });
                } else {
                    operations.push(TagOperation {
                        raw: raw_value,
                        name,
                        intro,
                        links,
                        operation: TagAction::Skip,
                    });
                }
            } else {
                operations.push(TagOperation {
                    raw: raw_value,
                    name,
                    intro,
                    links,
                    operation: TagAction::Insert,
                });
            }
        }

        operations
    }

    pub fn get_all_tags(&mut self) -> Result<HashMap<String, HashMap<String, String>>> {
        let mut all_tags = HashMap::new();

        for namespace in NAMESPACES {
            let table_name = if *namespace == "group" {
                "groups"
            } else {
                namespace
            };

            let dyn_table = table(table_name);
            let raw_col = dyn_table.column::<Text, _>("raw");
            let name_col = dyn_table.column::<Text, _>("name");

            let results: Vec<(String, String)> =
                dyn_table
                    .select((raw_col, name_col))
                    .load::<(String, String)>(&mut self.conn)?;

            let mut tags_map = HashMap::new();
            for (raw, name) in results {
                tags_map.insert(raw, name);
            }

            all_tags.insert(namespace.to_string(), tags_map);
        }

        Ok(all_tags)
    }
}
