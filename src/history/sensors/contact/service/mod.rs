use chrono::{DateTime, Utc};

use crate::history::sensors::{
    contact::repository::Repository, error::Result, model::VibrationCount,
};
use std::sync::Arc;

pub struct ContactSensorHistoryService {
    repository: Arc<Repository>,
}

impl ContactSensorHistoryService {
    /// Creates a new instance of the service with an InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - InfluxDB client
    /// * `bucket` - Name of the InfluxDB bucket history writes target,
    ///   taken from the configuration file
    ///
    /// # Returns
    ///
    /// A new instance of the service initialized with the provided client
    /// and bucket name.
    pub fn new(client: Arc<influxdb2::Client>, bucket: String) -> Self {
        let repository = Arc::new(Repository::new(client, bucket));
        Self { repository }
    }
}

impl ContactSensorHistoryService {
    /// Records history data for a contact sensor.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - The sensor's unique identifier
    /// * `sensor_mac` - The sensor's MAC address
    /// * `is_open` - Whether the sensor detects an open state
    /// * `vibration_count` - Optional vibration count measurement
    /// * `sensor_timestamp` - Optional timestamp from the sensor
    ///
    /// # Returns
    ///
    /// * `Ok(())` - History data recorded successfully
    /// * `Err(SensorHistoryError)` - If recording fails
    pub async fn set_history(
        &self,
        sensor_id: u32,
        sensor_mac: String,
        is_open: bool,
        vibration_count: Option<VibrationCount>,
        sensor_timestamp: Option<DateTime<Utc>>,
    ) -> Result<()> {
        self.repository
            .set_history(
                sensor_id,
                sensor_mac,
                is_open,
                vibration_count,
                sensor_timestamp,
            )
            .await
    }
}
