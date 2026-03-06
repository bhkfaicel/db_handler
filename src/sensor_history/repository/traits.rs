use crate::sensor_history::{error::Result, model::SensorDataPoint};

/// Trait for time-series database operations.
///
/// This trait abstracts the operations needed to store and query
/// sensor historical data, allowing easy switching between different
/// time-series databases (InfluxDB, TimescaleDB, QuestDB, etc.).
///
/// # Example
///
/// no_run
/// # // Note: sensor_history is pub(crate), so this example uses no_run
/// # use crate::sensor_history::repository::traits::TimeSeriesDatabase;
/// # use crate::sensor_history::model::SensorDataPoint;
/// #
/// # async fn example<T: TimeSeriesDatabase>(db: T) -> Result<(), Box<dyn std::error::Error>> {
/// # // Create a point (example code)
/// # // let point = SensorDataPoint::new(...);
/// # // db.write_point(&point).await?;
/// # Ok(())
/// # }
///
#[async_trait::async_trait]
pub trait TimeSeriesDatabase: Send + Sync {
    /// Writes a single data point to the time-series database.
    ///
    /// # Arguments
    ///
    /// * `point` - Sensor data point to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data point written successfully
    /// * `Err(SensorHistoryError)` - If write operation fails
    async fn write_point(&self, point: &SensorDataPoint) -> Result<()>;

    /// Writes multiple data points to the time-series database in a single batch.
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
    /// This method should be more efficient than writing points individually
    /// as it batches multiple writes into a single operation.
    async fn write_batch(&self, points: &[SensorDataPoint]) -> Result<()>;

    /// Queries the time-series database using the provided query string.
    ///
    /// # Arguments
    ///
    /// * `query` - Query string in the database's query language
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Query result (format depends on implementation, typically CSV or JSON)
    /// * `Err(SensorHistoryError)` - If query operation fails
    ///
    /// The query language depends on the implementation:
    /// - InfluxDB: Flux query language
    /// - TimescaleDB: SQL
    /// - QuestDB: SQL
    async fn query(&self, query: &str) -> Result<String>;
}
