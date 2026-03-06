use chrono::{DateTime, Utc};

use crate::{
    db::history::{repository::HistoryDbRepository, service::HistoryDbService},
    history::sensors::{contact::repository::Repository, error::Result, model::VibrationCount},
};
use std::sync::Arc;
pub struct ContactSensorHistoryService {
    repository: Arc<Repository>,
}
impl HistoryDbService for ContactSensorHistoryService {
    fn new(client: Arc<influxdb2::Client>) -> Self {
        let repository = Arc::new(Repository::new(client.clone()));
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
