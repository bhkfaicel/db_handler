use crate::sensor_history::{
    error::Result,
    model::{SensorDataPoint, SensorFields, SensorTags},
    repository::{influxdb_impl::InfluxDbBackend, SensorHistoryRepository},
};
use crate::sensors::model::ProductType;
use influxdb2::Client;
use std::sync::Arc;
/// Service for managing sensor history data.
///
/// This service provides high-level business logic for storing and
/// retrieving sensor historical data in time-series databases. It acts as a
/// facade over the repository layer, which uses a generic database abstraction.
pub struct SensorHistoryService {
    repository: Arc<SensorHistoryRepository>,
}

impl SensorHistoryService {
    /// Creates a new service with the given InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - Time-series database client instance
    ///
    /// # Returns
    ///
    /// A new `SensorHistoryService` instance.
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::connection::get_time_series_database_client;
    /// # use crate::sensor_history::service::SensorHistoryService;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = get_time_series_database_client().await?;
    /// # let service = SensorHistoryService::new(Arc::new(client));
    /// # Ok(())
    /// # }
    ///
    pub fn new(client: Arc<Client>) -> Self {
        let backend = Arc::new(InfluxDbBackend::new(client));
        let repository = Arc::new(SensorHistoryRepository::new(backend));
        Self { repository }
    }

    /// Writes a contact sensor data point to the time-series database.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - Unique sensor identifier
    /// * `sensor_mac` - Sensor MAC address
    /// * `is_open` - Whether the sensor detects an open state
    /// * `vibration_count` - Optional vibration count measurement
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
    /// # use crate::sensor_history::{
    /// #     model::VibrationCount,
    /// #     service::SensorHistoryService,
    /// # };
    /// # use crate::sensor_history::model::TimeUnit;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let service = SensorHistoryService::new(client);
    /// #
    /// # // Without vibration count
    /// # service.write_contact_sensor_data(123, "AA:BB:CC:DD:EE:01".to_string(), true, None).await?;
    /// #
    /// # // With vibration count
    /// # let vibration = Some(VibrationCount {
    /// #     count: 5,
    /// #     time_window: 10,
    /// #     unit: TimeUnit::Seconds,
    /// # });
    /// # service.write_contact_sensor_data(123, "AA:BB:CC:DD:EE:01".to_string(), true, vibration).await?;
    /// # Ok(())
    /// # }
    ///
    pub async fn write_contact_sensor_data(
        &self,
        sensor_id: u32,
        sensor_mac: String,
        is_open: bool,
        vibration_count: Option<crate::sensor_history::model::VibrationCount>,
    ) -> Result<()> {
        let tags = SensorTags::new(ProductType::ContactSensor, sensor_id, sensor_mac);
        let fields = SensorFields::Contact {
            is_open,
            vibration_count,
        };
        let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

        self.repository.write_point(&point).await
    }

    /// Writes a temperature sensor data point to InfluxDB.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - Unique sensor identifier
    /// * `sensor_mac` - Sensor MAC address
    /// * `temperature` - Temperature value in Celsius
    /// * `humidity` - Optional humidity value in percentage
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
    /// # use crate::sensor_history::service::SensorHistoryService;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let service = SensorHistoryService::new(client);
    /// # service.write_temperature_sensor_data(
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// #     22.5,
    /// #     Some(45.0),
    /// # ).await?;
    /// # Ok(())
    /// # }
    ///
    pub async fn write_temperature_sensor_data(
        &self,
        sensor_id: u32,
        sensor_mac: String,
        temperature: f64,
        humidity: Option<f64>,
    ) -> Result<()> {
        // Note: This should use TemperatureSensor when ProductType enum is extended
        let tags = SensorTags::new(ProductType::ContactSensor, sensor_id, sensor_mac);
        let fields = SensorFields::Temperature {
            temperature,
            humidity,
        };
        let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

        self.repository.write_point(&point).await
    }

    /// Writes a vibration sensor data point to InfluxDB.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - Unique sensor identifier
    /// * `sensor_mac` - Sensor MAC address
    /// * `vibration_level` - Vibration level (0.0 to 1.0)
    /// * `frequency` - Frequency in Hz
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data point written successfully
    /// * `Err(SensorHistoryError)` - If write operation fails
    pub async fn write_vibration_sensor_data(
        &self,
        sensor_id: u32,
        sensor_mac: String,
        vibration_level: f64,
        frequency: f64,
    ) -> Result<()> {
        let tags = SensorTags::new(ProductType::ContactSensor, sensor_id, sensor_mac);
        let fields = SensorFields::Vibration {
            vibration_level,
            frequency,
        };
        let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

        self.repository.write_point(&point).await
    }

    /// Writes a batch of sensor data points to InfluxDB.
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
    /// This method is more efficient than writing points individually
    /// as it batches multiple writes into a single HTTP request.
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::sensor_history::{
    /// #     model::{SensorDataPoint, SensorFields, SensorTags},
    /// #     service::SensorHistoryService,
    /// # };
    /// # use crate::sensors::model::ProductType;
    /// # use crate::db::connection::Connection;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let service = SensorHistoryService::new(client);
    /// #
    /// # let mut points = Vec::new();
    /// # for i in 0..10 {
    /// #     let tags = SensorTags::new(
    /// #         ProductType::ContactSensor,
    /// #         i,
    /// #         format!("AA:BB:CC:DD:EE:{:02X}", i),
    /// #     );
    /// #     let fields = SensorFields::Contact { is_open: i % 2 == 0 };
    /// #     let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);
    /// #     points.push(point);
    /// # }
    /// #
    /// # service.write_batch(&points).await?;
    /// # Ok(())
    /// # }
    ///
    pub async fn write_batch(&self, points: &[SensorDataPoint]) -> Result<()> {
        self.repository.write_batch(points).await
    }

    /// Queries sensor history data using Flux query language.
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
    /// # use crate::sensor_history::service::SensorHistoryService;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let service = SensorHistoryService::new(client);
    /// #
    /// # let query = r#"
    /// #     from(bucket: "iot_data")
    /// #       |> range(start: -1h)
    /// #       |> filter(fn: (r) => r["_measurement"] == "sensor_data")
    /// #       |> filter(fn: (r) => r["product_type"] == "ContactSensor")
    /// # "#;
    /// #
    /// # let result = service.query_flux(query).await?;
    /// # Ok(())
    /// # }
    ///
    pub async fn query_flux(&self, flux_query: &str) -> Result<String> {
        self.repository.query_flux(flux_query).await
    }

    /// Queries sensor history data for a specific sensor within a time range.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - Unique sensor identifier
    /// * `start_time` - Start time for the query (RFC3339 format or relative like "-1h")
    /// * `end_time` - End time for the query (RFC3339 format or "now")
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
    /// # use crate::sensor_history::service::SensorHistoryService;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let client = connection.history_db_client();
    /// # let service = SensorHistoryService::new(client);
    /// # let result = service.query_sensor_history(123, "-24h", "now").await?;
    /// # Ok(())
    /// # }
    ///
    pub async fn query_sensor_history(
        &self,
        sensor_id: u32,
        start_time: &str,
        end_time: &str,
    ) -> Result<String> {
        // Note: This query is InfluxDB-specific (Flux syntax)
        // For other databases, you would need to adapt the query syntax
        let query = format!(
            r#"
            from(bucket: "iot_data")
              |> range(start: {}, stop: {})
              |> filter(fn: (r) => r["_measurement"] == "sensor_data")
              |> filter(fn: (r) => r["sensor_id"] == "{}")
            "#,
            start_time, end_time, sensor_id
        );

        self.query_flux(&query).await
    }
}

#[cfg(test)]
mod tests;
