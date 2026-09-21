//! Battery telemetry, independent of door state. SQLite commits before InfluxDB.
use crate::db::primary::error::CommonError;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::db::{connection::Connection, history::connection::HistoryDbClient};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use thiserror::Error;

/// Latest battery state of a sensor: resting and loaded voltage.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BatteryReading {
    /// Identifier of the sensor the reading belongs to.
    pub sensor_id: u32,
    /// Battery voltage in millivolts at rest.
    pub resting_mv: u16,
    /// Battery voltage in millivolts under load.
    pub loaded_mv: u16,
    /// Time the reading was received by the server.
    pub received_at: DateTime<Utc>,
}

/// Errors that can occur when recording or reading battery telemetry.
#[derive(Debug, Error)]
pub enum BatteryError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation).
    #[error(transparent)]
    Common(#[from] CommonError),

    /// History write failure. The SQLite latest value is already committed
    /// when this error surfaces, so the local state stays consistent.
    #[error("Failed to write battery history: {0}")]
    HistoryWrite(String),

    /// The reading's timestamp cannot be represented with the nanosecond
    /// precision the InfluxDB line protocol requires.
    #[error("Invalid battery timestamp: {0}")]
    InvalidTimestamp(String),
}

/// Result type for battery telemetry operations.
pub type Result<T> = std::result::Result<T, BatteryError>;

impl BatteryError {
    /// Creates a new history write error.
    pub fn history_write(msg: impl Into<String>) -> Self {
        Self::HistoryWrite(msg.into())
    }

    /// Creates a new invalid timestamp error.
    pub fn invalid_timestamp(msg: impl Into<String>) -> Self {
        Self::InvalidTimestamp(msg.into())
    }
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

    /// Returns the latest battery reading stored for a sensor.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - The sensor's unique identifier
    ///
    /// # Returns
    ///
    /// The sensor's most recent battery reading, or `None` when the sensor
    /// has no stored reading (including unknown sensor identifiers).
    pub async fn latest(&self, sensor_id: u32) -> Result<Option<BatteryReading>> {
        Ok(query_latest(self.primary.as_ref(), sensor_id).await?)
    }

    /// Persist the last value even if the subsequent historical write fails.
    /// Older deliveries may enter history but cannot roll back the last value.
    ///
    /// # Arguments
    ///
    /// * `reading` - The battery reading to persist
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Reading committed to SQLite and written to history
    /// * `Err(BatteryError::Common)` - If the SQLite upsert fails (for
    ///   example an orphan reading for an unknown sensor)
    /// * `Err(BatteryError::HistoryWrite)` - If the history write fails; the
    ///   SQLite latest value is still committed
    /// * `Err(BatteryError::InvalidTimestamp)` - If the reading's timestamp
    ///   cannot be rendered for the history line protocol
    pub async fn record(&self, reading: &BatteryReading) -> Result<()> {
        let line = history_line(reading)?;
        upsert_latest(self.primary.as_ref(), reading).await?;
        self.history
            .write(&self.bucket, line)
            .await
            .map_err(|error| {
                BatteryError::history_write(format!(
                    "battery history write failed (SQLite latest value committed): {error}"
                ))
            })
    }
}

// SQLite-only helper: fetches the latest reading of a sensor. Returns the
// crate's common error type so the single sqlx conversion point stays
// authoritative; the service boundary converts it into `BatteryError`
// through its `#[from]` impl.
async fn query_latest(
    pool: &SqlitePool,
    sensor_id: u32,
) -> crate::db::primary::error::Result<Option<BatteryReading>> {
    sqlx::query_as("SELECT sensor_id, resting_mv, loaded_mv, received_at FROM sensor_battery WHERE sensor_id = ?")
        .bind(sensor_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_to_common_error)
}

// SQLite-only helper: upserts the latest reading, keeping the newest
// reception (idempotent on equal timestamps). Same `CommonError` routing
// as `query_latest`.
async fn upsert_latest(
    pool: &SqlitePool,
    reading: &BatteryReading,
) -> crate::db::primary::error::Result<()> {
    sqlx::query("INSERT INTO sensor_battery (sensor_id, resting_mv, loaded_mv, received_at) VALUES (?, ?, ?, ?) \
        ON CONFLICT(sensor_id) DO UPDATE SET resting_mv = excluded.resting_mv, loaded_mv = excluded.loaded_mv, received_at = excluded.received_at \
        WHERE excluded.received_at >= sensor_battery.received_at")
        .bind(reading.sensor_id)
        .bind(reading.resting_mv)
        .bind(reading.loaded_mv)
        .bind(reading.received_at)
        .execute(pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
    Ok(())
}

// Renders the InfluxDB line-protocol body for a battery reading.
fn history_line(reading: &BatteryReading) -> Result<String> {
    let timestamp = reading.received_at.timestamp_nanos_opt().ok_or_else(|| {
        BatteryError::invalid_timestamp(format!(
            "battery timestamp outside InfluxDB nanosecond range: {}",
            reading.received_at
        ))
    })?;
    Ok(format!(
        "sensor_battery,sensor_id={} resting_mv={}i,loaded_mv={}i {}",
        reading.sensor_id, reading.resting_mv, reading.loaded_mv, timestamp
    ))
}

#[cfg(test)]
mod tests;
