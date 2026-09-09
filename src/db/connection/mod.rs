use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

use crate::db::history::connection::{get_time_series_database_client, HistoryDbClient};
use crate::db::primary::connection::get_primary_db_client;
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
pub struct Connection {
    primary_db_client: Arc<SqlitePool>,
    history_db_client: Arc<HistoryDbClient>,
    history_db_bucket: String,
}

impl Connection {
    pub async fn new() -> Result<Self> {
        let primary_db_client = get_primary_db_client()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get primary database client: {}", e))?;
        // Production defaults: 5 s to establish the connection, 30 s for a
        // complete request. Without them a hung/unreachable InfluxDB would
        // stall history writes indefinitely (the factory documents the
        // scenario); production never waits unbounded.
        let (history_db_client, history_db_bucket) =
            get_time_series_database_client(Duration::from_secs(5), Duration::from_secs(30))
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

    pub fn history_db_client(&self) -> Arc<HistoryDbClient> {
        self.history_db_client.clone()
    }

    /// Returns the InfluxDB bucket name that history writes target, as
    /// configured in the configuration file.
    pub fn history_db_bucket(&self) -> String {
        self.history_db_bucket.clone()
    }
}
