use crate::{
    db::primary::{
        error::{CommonError, Result},
        repository::PrimaryDbRepository,
        service::map_sqlx_to_common_error,
    },
    sensors::{
        contact::model::ContactSensor,
        model::{Sensor, Status},
    },
};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;

/// Repository for contact-sensor rows in the primary database.
///
/// Methods return `CommonError` (converted at the single central point), so
/// the service never needs to inspect raw sqlx or anyhow error payloads.
pub(crate) struct ContactSensorRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for ContactSensorRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl ContactSensorRepository {
    // Helper to convert Option<Option<i32>> to Option<bool> (for get_by_id)
    fn convert_is_open_nested(is_open: Option<Option<i32>>) -> Option<bool> {
        is_open.and_then(|v| v.map(|val| val != 0))
    }

    // Helper to convert Option<i32> to Option<bool> (for get_by_mac)
    fn convert_is_open_simple(is_open: Option<i32>) -> Option<bool> {
        is_open.map(|v| v != 0)
    }

    // Helper to flatten Option<Option<NaiveDateTime>> to Option<NaiveDateTime> (for get_by_id)
    fn flatten_updated_at_nested(
        updated_at: Option<Option<NaiveDateTime>>,
    ) -> Option<NaiveDateTime> {
        updated_at.and_then(|v| v)
    }

    // Retrieves a contact sensor and its values by ID
    pub async fn get_by_id(&self, id: u32) -> Result<Option<ContactSensor>> {
        let result = sqlx::query!(
            r#"
        SELECT s.id, s.mac, s.status,
         ocs.is_open as "is_open: Option<i32>", 
        ocs.updated_at as "updated_at: Option<NaiveDateTime>"
        FROM sensors s
        LEFT JOIN open_close_sensors ocs ON s.id = ocs.sensor_id
        WHERE s.id = ? AND s.sensor_type = 'ContactSensor'
        "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;

        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| CommonError::Operation("Invalid status value".to_string()))?;
            // Handle Option<Option<i32>> from LEFT JOIN (get_by_id)
            let is_open = Self::convert_is_open_nested(record.is_open);
            // Handle Option<Option<NaiveDateTime>> from LEFT JOIN (get_by_id)
            let updated_at = Self::flatten_updated_at_nested(record.updated_at);
            Ok(Some(ContactSensor::new(
                Sensor::new(record.id as u32, record.mac, status),
                is_open,
                updated_at,
            )))
        } else {
            Ok(None)
        }
    }
    /// Retrieves a contact sensor by its MAC address
    pub async fn get_by_mac(&self, mac: &str) -> Result<Option<ContactSensor>> {
        let result = sqlx::query!(
            r#"
        SELECT s.id, s.mac, s.status,
         ocs.is_open as "is_open: Option<i32>", 
        ocs.updated_at as "updated_at: Option<NaiveDateTime>"
        FROM sensors s
        LEFT JOIN open_close_sensors ocs ON s.mac = ocs.sensor_mac
        WHERE s.mac = ? AND s.sensor_type = 'ContactSensor'
        "#,
            mac
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;

        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| CommonError::Operation("Invalid status value".to_string()))?;
            let id = record
                .id
                .ok_or_else(|| CommonError::Operation("ID is null".to_string()))?
                as u32;
            // Handle Option<i32> from LEFT JOIN (get_by_mac)
            let is_open = Self::convert_is_open_simple(record.is_open);
            // Handle Option<NaiveDateTime> from LEFT JOIN (get_by_mac)
            let updated_at = record.updated_at;
            Ok(Some(ContactSensor::new(
                Sensor::new(id, record.mac, status),
                is_open,
                updated_at,
            )))
        } else {
            Ok(None)
        }
    }
    /// Updates a contact sensor by its MAC address
    ///
    /// # Arguments
    ///
    /// * `mac` - The MAC address of the sensor to update
    /// * `is_open` - The new value for the is_open field
    /// * `updated_at` - The new value for the updated_at field
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - The number of rows updated
    /// * `Err(CommonError)` - If an error occurs during the update (e.g. the
    ///   referenced sensor does not exist: foreign-key violation)
    pub async fn update_by_mac(
        &self,
        id: u32,
        mac: &str,
        is_open: bool,
        updated_at: NaiveDateTime,
    ) -> Result<u32> {
        let is_open_int = if is_open { 1 } else { 0 };

        let result = sqlx::query!(
            r#"
    INSERT INTO open_close_sensors (sensor_id, sensor_mac, is_open, updated_at)
    VALUES (?, ?, ?,  ?)
    ON CONFLICT(sensor_id) DO UPDATE SET
      is_open    = excluded.is_open,
      updated_at = excluded.updated_at
    "#,
            id,
            mac,
            is_open_int,
            updated_at
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.rows_affected() as u32)
    }
}

#[cfg(test)]
mod tests;
