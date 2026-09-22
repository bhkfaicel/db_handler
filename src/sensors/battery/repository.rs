//! Persistence for battery telemetry: SQLite latest-value store and
//! InfluxDB history writes, following the crate's dual-store repositories.

use std::sync::Arc;

use crate::db::history::connection::HistoryDbClient;
use crate::db::primary::service::map_sqlx_to_common_error;
use sqlx::SqlitePool;

use super::error::{BatteryError, Result};
use super::model::BatteryReading;

/// Dual-store repository for battery telemetry: `record` commits the latest
/// value to SQLite first, then appends the reading to the InfluxDB history
/// bucket; `latest` only reads the SQLite store.
pub(crate) struct BatteryRepository {
    primary: Arc<SqlitePool>,
    history: Arc<HistoryDbClient>,
    bucket: String,
}

impl BatteryRepository {
    /// Creates a new instance of the repository with its database parts.
    ///
    /// # Arguments
    ///
    /// * `primary` - Primary SQLite connection pool
    /// * `history` - History database client
    /// * `bucket` - Name of the InfluxDB bucket history writes target,
    ///   taken from the configuration file
    ///
    /// # Returns
    ///
    /// A new instance of the repository initialized with the provided
    /// pool, client and bucket name.
    pub(crate) fn new(
        primary: Arc<SqlitePool>,
        history: Arc<HistoryDbClient>,
        bucket: String,
    ) -> Self {
        Self {
            primary,
            history,
            bucket,
        }
    }

    /// Fetches the latest reading of a sensor from the SQLite store.
    pub(crate) async fn latest(
        &self,
        sensor_id: u32,
    ) -> crate::db::primary::error::Result<Option<BatteryReading>> {
        sqlx::query_as("SELECT sensor_id, resting_mv, loaded_mv, received_at FROM sensor_battery WHERE sensor_id = ?")
            .bind(sensor_id)
            .fetch_optional(self.primary.as_ref())
            .await
            .map_err(map_sqlx_to_common_error)
    }

    /// Commits the latest reading to SQLite, then writes the history line.
    /// The SQLite value is committed before the history write is attempted,
    /// so a history failure surfaces as `BatteryError::HistoryWrite` while
    /// the local latest value stays in place.
    pub(crate) async fn record(&self, reading: &BatteryReading) -> Result<()> {
        let line = history_line(reading)?;
        self.upsert_latest(reading).await?;
        // ponytail: history write awaited inline; caller latency couples to the InfluxDB round-trip (bounded by the 30 s request timeout). Detach or batch writes if battery reporting frequency grows.
        self.history
            .write(&self.bucket, line)
            .await
            .map_err(|error| {
                BatteryError::history_write(format!(
                    "battery history write failed (SQLite latest value committed): {error}"
                ))
            })
    }

    // SQLite-only helper: upserts the latest reading, keeping the newest
    // reception (idempotent on equal timestamps). Returns the crate's
    // common error type so the single sqlx conversion point stays
    // authoritative, same routing as `latest`.
    async fn upsert_latest(
        &self,
        reading: &BatteryReading,
    ) -> crate::db::primary::error::Result<()> {
        sqlx::query("INSERT INTO sensor_battery (sensor_id, resting_mv, loaded_mv, received_at) VALUES (?, ?, ?, ?) \
            ON CONFLICT(sensor_id) DO UPDATE SET resting_mv = excluded.resting_mv, loaded_mv = excluded.loaded_mv, received_at = excluded.received_at \
            WHERE excluded.received_at >= sensor_battery.received_at")
            .bind(reading.sensor_id)
            .bind(reading.resting_mv)
            .bind(reading.loaded_mv)
            .bind(reading.received_at)
            .execute(self.primary.as_ref())
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
}

// Renders the InfluxDB line-protocol body for a battery reading.
//
// Invariant: every interpolated input is integer-only (`sensor_id`, the two
// u16 voltages, the nanosecond timestamp), so no InfluxDB line-protocol
// escaping is needed today. Any future string tag or field MUST be escaped
// per InfluxDB line-protocol rules, as the contact repository's private
// line renderer implements.
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
