//! Battery telemetry, independent of door state. SQLite commits before InfluxDB.
use crate::db::{connection::Connection, history::connection::HistoryDbClient};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BatteryReading {
    pub sensor_id: u32,
    pub resting_mv: u16,
    pub loaded_mv: u16,
    pub received_at: DateTime<Utc>,
}

pub struct BatteryService {
    primary: Arc<SqlitePool>,
    history: Arc<HistoryDbClient>,
    bucket: String,
}

impl BatteryService {
    pub fn new(connection: Connection) -> Self {
        Self {
            primary: connection.primary_db_client(),
            history: connection.history_db_client(),
            bucket: connection.history_db_bucket(),
        }
    }

    pub async fn latest(&self, sensor_id: u32) -> Result<Option<BatteryReading>> {
        Ok(sqlx::query_as("SELECT sensor_id, resting_mv, loaded_mv, received_at FROM sensor_battery WHERE sensor_id = ?")
            .bind(sensor_id).fetch_optional(self.primary.as_ref()).await?)
    }

    /// Persist the last value even if the subsequent historical write fails.
    /// Older deliveries may enter history but cannot roll back the last value.
    pub async fn record(&self, reading: &BatteryReading) -> Result<()> {
        let line = history_line(reading)?;
        sqlx::query("INSERT INTO sensor_battery (sensor_id, resting_mv, loaded_mv, received_at) VALUES (?, ?, ?, ?) \
            ON CONFLICT(sensor_id) DO UPDATE SET resting_mv = excluded.resting_mv, loaded_mv = excluded.loaded_mv, received_at = excluded.received_at \
            WHERE excluded.received_at >= sensor_battery.received_at")
            .bind(reading.sensor_id).bind(reading.resting_mv).bind(reading.loaded_mv)
            .bind(reading.received_at).execute(self.primary.as_ref()).await?;
        self.history
            .write(&self.bucket, line)
            .await
            .context("battery history write failed (SQLite latest value committed)")
    }
}

fn history_line(reading: &BatteryReading) -> Result<String> {
    let timestamp = reading
        .received_at
        .timestamp_nanos_opt()
        .context("battery timestamp outside InfluxDB range")?;
    Ok(format!(
        "sensor_battery,sensor_id={} resting_mv={}i,loaded_mv={}i {}",
        reading.sensor_id, reading.resting_mv, reading.loaded_mv, timestamp
    ))
}

#[cfg(test)]
mod tests;
