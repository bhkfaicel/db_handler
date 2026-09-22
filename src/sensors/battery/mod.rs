//! Battery telemetry, independent of door state. SQLite commits before InfluxDB.

mod error;
mod model;
mod repository;

// Public surface of the battery domain: every item a caller may name stays
// reachable under `sensors::battery::`, exactly as in the pre-split module.
pub use self::error::{BatteryError, Result};
pub use self::model::BatteryReading;

use crate::db::connection::Connection;
use crate::db::primary::service::Service;

use self::repository::BatteryRepository;

// The parts constructor below exists only for the tests, so its signature
// types are compiled exclusively with the test configuration.
#[cfg(test)]
use crate::db::history::connection::HistoryDbClient;
#[cfg(test)]
use sqlx::SqlitePool;
#[cfg(test)]
use std::sync::Arc;

/// Service exposing the battery telemetry of a sensor: it keeps the latest
/// reading of each sensor in the primary SQLite database and appends every
/// reception to the InfluxDB history bucket.
pub struct BatteryService {
    repository: BatteryRepository,
}

/// Implements the crate's uniform service contract. The `Service` trait must
/// be in scope to construct the service (`BatteryService::new(connection)`
/// only resolves with the trait imported, e.g. through the prelude), exactly
/// like every other service of the crate.
impl Service for BatteryService {
    fn new(connection: Connection) -> Self {
        Self {
            repository: BatteryRepository::new(
                connection.primary_db_client(),
                connection.history_db_client(),
                connection.history_db_bucket(),
            ),
        }
    }
}

impl BatteryService {
    // Crate-internal constructor reserved for the battery tests: each test
    // isolates its data in a private in-memory SQLite pool and points the
    // history client at a local fake endpoint, neither of which the public
    // `Service` constructor accepts.
    #[cfg(test)]
    pub(crate) fn with_parts(
        primary: Arc<SqlitePool>,
        history: Arc<HistoryDbClient>,
        bucket: String,
    ) -> Self {
        Self {
            repository: BatteryRepository::new(primary, history, bucket),
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
        Ok(self.repository.latest(sensor_id).await?)
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
        self.repository.record(reading).await
    }
}

#[cfg(test)]
mod tests;
