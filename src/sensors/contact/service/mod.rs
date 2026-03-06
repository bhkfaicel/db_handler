use std::sync::Arc;

use crate::db::history::service::HistoryDbService;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::Service;
use crate::history::{
    sensors::contact::service::ContactSensorHistoryService,
    sensors::error::map_history_error_to_anyhow, sensors::model::VibrationCount,
};

use crate::sensors::contact::error::ContactSensorError;
use crate::sensors::contact::error::Result;
use crate::sensors::contact::model::ContactSensor;
use crate::sensors::model::TypedSensorTrait;
use crate::{db::connection::Connection, sensors::contact::repository::ContactSensorRepository};
use chrono::{DateTime, Utc};
pub struct ContactSensorService {
    primary_repository: Arc<ContactSensorRepository>,
    history_service: Arc<ContactSensorHistoryService>,
}
impl Service for ContactSensorService {
    fn new(connection: Connection) -> Self {
        let repository = ContactSensorRepository::new(connection.primary_db_client());
        let history_service = ContactSensorHistoryService::new(connection.history_db_client());
        Self {
            primary_repository: Arc::new(repository),
            history_service: Arc::new(history_service),
        }
    }
}
impl ContactSensorService {
    pub async fn get_by_id(&self, id: u32) -> Result<Option<ContactSensor>> {
        self.primary_repository
            .get_by_id(id)
            .await
            .map_err(|_e| ContactSensorError::not_found(id.to_string()))
    }

    pub async fn get_by_mac(&self, mac: &str) -> Result<Option<ContactSensor>> {
        self.primary_repository
            .get_by_mac(mac)
            .await
            .map_err(|_e| ContactSensorError::not_found(mac))
    }

    /// Records history data for a contact sensor.
    ///
    /// This method takes a `ContactSensor` instance and records its history data.
    /// The sensor's ID and MAC address are extracted automatically from the sensor.
    ///
    /// # Arguments
    ///
    /// * `sensor` - The contact sensor to record history for
    /// * `is_open` - Whether the sensor detects an open state
    /// * `vibration_count` - Optional vibration count measurement
    /// * `sensor_timestamp` - Optional timestamp from the sensor
    ///
    /// # Returns
    ///
    /// * `Ok(())` - History data recorded successfully
    /// * `Err(anyhow::Error)` - If recording fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::db_handler::sensors::contact::service::ContactSensorService;
    /// # use crate::db_handler::db::connection::Connection;
    /// # use crate::db_handler::prelude::*;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let connection = Connection::new().await?;
    /// # let service = ContactSensorService::new(connection);
    /// # let sensor = service.get_by_id(1).await?.unwrap();
    /// service.set_history(
    ///     &sensor,
    ///     true,
    ///     None,
    ///     None,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_history(
        &self,
        sensor: &ContactSensor,
        is_open: bool,
        vibration_count: Option<VibrationCount>,
        sensor_timestamp: Option<DateTime<Utc>>,
    ) -> Result<()> {
        log::debug!(
            "ContactSensorService::set_history called - sensor_id: {}, sensor_mac: {}, is_open: {}",
            sensor.id(),
            sensor.mac(),
            is_open
        );
        match self
            .primary_repository
            .update_by_mac(
                sensor.id(),
                &sensor.mac().to_string(),
                is_open,
                Utc::now().naive_utc(),
            )
            .await
        {
            Ok(_) => {
                match self
                    .history_service
                    .set_history(
                        sensor.id(),
                        sensor.mac().to_string(),
                        is_open,
                        vibration_count,
                        sensor_timestamp,
                    )
                    .await
                {
                    Ok(_) => {
                        log::debug!(
                    "ContactSensorService::set_history completed successfully - sensor_id: {}, sensor_mac: {}",
                    sensor.id(),
                    sensor.mac()
                );
                        Ok(())
                    }
                    Err(e) => {
                        let error = map_history_error_to_anyhow(e);
                        log::error!(
                    "ContactSensorService::set_history failed - sensor_id: {}, sensor_mac: {}, error: {:?}",
                    sensor.id(),
                    sensor.mac(),
                    error
                );
                        Err(ContactSensorError::operation(error.to_string()))
                    }
                }
            }
            Err(_e) => {
                let error = ContactSensorError::not_found(sensor.mac().to_string());
                log::error!(
                    "ContactSensorService::set_history failed to update_by_mac - sensor_id: {}, sensor_mac: {}, error: {:?}",
                    sensor.id(),
                    sensor.mac(),
                    error
                );
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests;
