pub mod influxdb_impl;
mod traits;

pub use traits::TimeSeriesDatabase;

use crate::sensor_history::{error::Result, model::SensorDataPoint};
use std::sync::Arc;

/// Repository for performing operations on sensor history data.
///
/// This repository uses a generic time-series database backend,
/// allowing easy switching between different implementations
/// (InfluxDB, TimescaleDB, QuestDB, etc.).
pub(crate) struct SensorHistoryRepository {
    db: Arc<dyn TimeSeriesDatabase>,
}

impl SensorHistoryRepository {
    /// Creates a new repository with the given time-series database implementation.
    ///
    /// # Arguments
    ///
    /// * `db` - Time-series database implementation
    ///
    /// # Returns
    ///
    /// A new `SensorHistoryRepository` instance.
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::db::connection::Connection;
    /// # use crate::sensor_history::repository::{SensorHistoryRepository, influxdb_impl::InfluxDbBackend};
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let backend = InfluxDbBackend::new(client);
    /// # let repo = SensorHistoryRepository::new(backend);
    /// # Ok(())
    /// # }
    ///
    pub(crate) fn new(db: Arc<dyn TimeSeriesDatabase>) -> Self {
        Self { db }
    }

    /// Writes a single data point to InfluxDB.
    ///
    /// # Arguments
    ///
    /// * `point` - Sensor data point to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data point written successfully
    /// * `Err(SensorHistoryError)` - If write operation fails
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::connection::get_time_series_database_client;
    /// # use crate::sensor_history::{
    /// #     model::{SensorDataPoint, SensorFields, SensorTags},
    /// #     repository::SensorHistoryRepository,
    /// # };
    /// # use crate::sensors::model::ProductType;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = get_time_series_database_client().await?;
    /// # let repo = SensorHistoryRepository::new(Arc::new(client));
    /// #
    /// # let tags = SensorTags::new(
    /// #     ProductType::ContactSensor,
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// # );
    /// #
    /// # let fields = SensorFields::Contact { is_open: true };
    /// # let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);
    /// #
    /// # repo.write_point(&point).await?;
    /// # Ok(())
    /// # }
    ///
    pub(crate) async fn write_point(&self, point: &SensorDataPoint) -> Result<()> {
        self.db.write_point(point).await
    }

    /// Writes multiple data points to InfluxDB in a single batch.
    ///
    /// # Arguments
    ///
    /// * `points` - Vector of sensor data points to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All data points written successfully
    /// * `Err(SensorHistoryError)` - If write operation fails
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::connection::get_time_series_database_client;
    /// # use crate::sensor_history::{
    /// #     model::{SensorDataPoint, SensorFields, SensorTags},
    /// #     repository::SensorHistoryRepository,
    /// # };
    /// # use crate::sensors::model::ProductType;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = get_time_series_database_client().await?;
    /// # let repo = SensorHistoryRepository::new(Arc::new(client));
    /// #
    /// # let mut points = Vec::new();
    /// # // ... add points to vector ...
    /// #
    /// # repo.write_batch(&points).await?;
    /// # Ok(())
    /// # }
    ///
    pub(crate) async fn write_batch(&self, points: &[SensorDataPoint]) -> Result<()> {
        self.db.write_batch(points).await
    }

    /// Queries the time-series database.
    ///
    /// # Arguments
    ///
    /// * `flux_query` - Flux query string
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Query result as CSV string
    /// * `Err(SensorHistoryError)` - If query operation fails
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::connection::get_time_series_database_client;
    /// # use crate::sensor_history::repository::SensorHistoryRepository;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = get_time_series_database_client().await?;
    /// # let repo = SensorHistoryRepository::new(Arc::new(client));
    /// #
    /// # let query = r#"
    /// #     from(bucket: "iot_data")
    /// #       |> range(start: -1h)
    /// #       |> filter(fn: (r) => r["_measurement"] == "sensor_data")
    /// # "#;
    /// #
    /// # let result = repo.query_flux(query).await?;
    /// # Ok(())
    /// # }
    ///
    pub(crate) async fn query_flux(&self, query: &str) -> Result<String> {
        self.db.query(query).await
    }
}

#[cfg(test)]
mod tests;
