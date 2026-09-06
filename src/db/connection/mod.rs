use anyhow::Result;
use std::sync::Arc;

use crate::db::history::connection::get_time_series_database_client;
use crate::db::primary::connection::get_primary_db_client;
use influxdb2::Client;
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
pub struct Connection {
    primary_db_client: Arc<SqlitePool>,
    history_db_client: Arc<Client>,
    history_db_bucket: String,
}

impl Connection {
    pub async fn new() -> Result<Self> {
        let primary_db_client = get_primary_db_client()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get primary database client: {}", e))?;
        let (history_db_client, history_db_bucket) = get_time_series_database_client()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get history database client: {}", e))?;
        Ok(Self {
            primary_db_client: Arc::new(primary_db_client),
            history_db_client: Arc::new(history_db_client),
            history_db_bucket,
        })
    }

    pub fn primary_db_client(&self) -> Arc<SqlitePool> {
        self.primary_db_client.clone()
    }

    pub fn history_db_client(&self) -> Arc<Client> {
        self.history_db_client.clone()
    }

    /// Returns the InfluxDB bucket name that history writes target, as
    /// configured in the configuration file.
    pub fn history_db_bucket(&self) -> String {
        self.history_db_bucket.clone()
    }
}
