use sqlx::error::ErrorKind;
use std::sync::Arc;

use crate::{
    db::{
        connection::Connection,
        primary::{
            models::{PaginatedResponse, PaginationParams as CommonPaginationParams},
            repository::PrimaryDbRepository,
            service::{map_sqlx_to_common_error, Service},
        },
    },
    sensors::{
        error::{Result, SensorError},
        model::{PaginationParams, ProductType, Sensor, Status, TypedSensor},
        repository::SensorRepository,
    },
};
/// Helper function to convert `sqlx::Error` to `SensorError`.
///
/// This function detects UNIQUE constraint violations (e.g., duplicate MAC) and converts
/// them to `SensorError::DuplicateMac`. Other database errors are converted via
/// the centralized `map_sqlx_to_common_error` function.
fn map_sqlx_error(e: sqlx::Error, mac: Option<&str>) -> SensorError {
    // Check for UNIQUE constraint violation (e.g., duplicate MAC)
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.kind() == ErrorKind::UniqueViolation {
            // Check if it's a MAC constraint violation
            let error_message = db_err.message();
            if error_message.contains("sensors.mac")
                || error_message.contains("UNIQUE constraint failed")
            {
                // Use provided MAC if available, otherwise try to extract from error message
                let mac_address = mac
                    .map(|m| m.to_string())
                    .or_else(|| extract_mac_from_error(error_message))
                    .unwrap_or_else(|| "duplicate entry".to_string());
                return SensorError::duplicate_mac(mac_address);
            }
        }
    }
    // For all other errors, use the centralized conversion
    map_sqlx_to_common_error(e).into()
}

// Helper function to convert anyhow::Error to SensorError
fn map_anyhow_to_sensor_error(e: anyhow::Error) -> SensorError {
    map_anyhow_to_sensor_error_with_mac(e, None)
}

// Helper function to convert anyhow::Error to SensorError with optional MAC for UNIQUE constraint errors
fn map_anyhow_to_sensor_error_with_mac(e: anyhow::Error, mac: Option<&str>) -> SensorError {
    // Try to extract sqlx::Error from anyhow::Error
    match e.downcast::<sqlx::Error>() {
        Ok(sqlx_err) => {
            // Use the centralized map_sqlx_error function
            map_sqlx_error(sqlx_err, mac)
        }
        Err(e) => {
            // For non-SQLx errors, use operation error
            SensorError::operation(e.to_string())
        }
    }
}

// Helper function to extract MAC address from error message
fn extract_mac_from_error(error_msg: &str) -> Option<String> {
    // SQLite UNIQUE constraint errors typically don't contain the actual value
    // But we can try to find MAC-like patterns in the message
    // Simple pattern matching without regex: look for "mac" keyword near the error
    if error_msg.to_lowercase().contains("mac") {
        // Try to find a MAC address pattern manually
        // Format: XX:XX:XX:XX:XX:XX or XX-XX-XX-XX-XX-XX
        let parts: Vec<&str> = error_msg.split_whitespace().collect();
        for part in parts {
            // Check if part looks like a MAC address (contains colons or dashes)
            if part.contains(':') || part.contains('-') {
                let chars: Vec<char> = part.chars().collect();
                // Simple check: should have 5 separators (colons or dashes)
                let separator_count = chars.iter().filter(|c| **c == ':' || **c == '-').count();
                if separator_count == 5 && part.len() >= 17 {
                    return Some(part.to_string());
                }
            }
        }
    }
    None
}

pub struct SensorService {
    pub(crate) repository: Arc<SensorRepository>,
}
impl Service for SensorService {
    fn new(connection: Connection) -> Self {
        let repository = SensorRepository::new(connection.primary_db_client().clone());
        Self {
            repository: Arc::new(repository),
        }
    }
}
impl SensorService {
    pub async fn add(&self, mac: &str) -> Result<u32> {
        if mac.is_empty() {
            return Err(SensorError::invalid_mac("MAC address cannot be empty"));
        }
        let sensor = Sensor::new(0, mac.to_string(), Status::InProgress);
        self.repository
            .insert(&sensor)
            .await
            .map_err(|e| map_anyhow_to_sensor_error_with_mac(e, Some(mac)))
    }

    /// Retrieves a sensor by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the sensor to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Sensor))` - If a sensor with the specified ID exists.
    /// * `Ok(None)` - If no sensor with the specified ID is found.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// if let Some(sensor) = service.get_by_id(1).await? {
    ///     println!("Sensor found: {:?}", sensor);
    /// } else {
    ///     println!("Sensor not found.");
    /// }
    /// # Ok(())
    /// # }
    /// ```

    pub async fn get_by_id(&self, id: u32) -> Result<Option<TypedSensor>> {
        let sensor = self
            .repository
            .get_by_id(id)
            .await
            .map_err(map_anyhow_to_sensor_error)?;

        let typed_sensor = sensor.map(TypedSensor::from_sensor);
        Ok(typed_sensor)
    }

    /// Retrieves a sensor by its mac.
    ///
    /// # Arguments
    ///
    /// * `mac` - The mac of the sensor to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Sensor))` - If a sensor with the specified ID exists.
    /// * `Ok(None)` - If no sensor with the specified ID is found.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// if let Some(sensor) = service.get_by_mac("mac").await? {
    ///     println!("Sensor found: {:?}", sensor);
    /// } else {
    ///     println!("Sensor not found.");
    /// }
    /// # Ok(())
    /// # }
    /// ```

    /// Gets a sensor by its MAC address, returning the typed sensor.
    ///
    /// # Arguments
    ///
    /// * `mac` - The MAC address of the sensor to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(TypedSensor))` - The sensor if found, with its specific type.
    /// * `Ok(None)` - If no sensor with the given MAC address exists.
    /// * `Err(SensorError)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// # if let Some(typed_sensor) = service.get_by_mac("00:1B:44:11:3A:B7").await? {
    /// #     // Use typed sensor methods
    /// #     // For contact sensors, you can access the sensor data via typed_sensor
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_mac(
        &self,
        mac: &str,
    ) -> Result<Option<crate::sensors::model::TypedSensor>> {
        let sensor = self
            .repository
            .get_by_mac(mac)
            .await
            .map_err(map_anyhow_to_sensor_error)?;

        let typed_sensor = sensor.map(crate::sensors::model::TypedSensor::from_sensor);

        Ok(typed_sensor)
    }

    /// Lists all sensors having exactly the provided status.
    ///
    /// # Arguments
    ///
    /// * `status` - The status to filter on.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Sensor>)` - Every matching sensor (empty vector if none).
    /// * `Err(SensorError)` - If a database error occurs.
    pub async fn list_by_status(&self, status: &Status) -> Result<Vec<Sensor>> {
        self.repository
            .list_by_status(status)
            .await
            .map_err(map_anyhow_to_sensor_error)
    }

    /// Gets a sensor by its MAC address, returning the raw sensor.
    ///
    /// This is a convenience method for cases where you need the raw `Sensor` type.
    ///
    /// # Arguments
    ///
    /// * `mac` - The MAC address of the sensor to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Sensor))` - The sensor if found.
    /// * `Ok(None)` - If no sensor with the given MAC address exists.
    /// * `Err(SensorError)` - If an error occurs during retrieval.
    pub async fn get_by_mac_raw(&self, mac: &str) -> Result<Option<Sensor>> {
        self.repository
            .get_by_mac(mac)
            .await
            .map_err(map_anyhow_to_sensor_error)
    }

    /// Deletes a sensor by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the sensor to delete.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the deletion was successful.
    /// * `Err(anyhow::Error)` - If an error occurs during deletion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// service.delete_by_id(1).await?;
    /// println!("Sensor deleted.");
    /// # Ok(())
    /// # }
    /// ```

    pub async fn delete_by_id(&self, id: u32) -> Result<()> {
        self.repository
            .delete_by_id(id)
            .await
            .map_err(map_anyhow_to_sensor_error)
    }

    /// Deletes a sensor by its Mac.
    ///
    /// # Arguments
    ///
    /// * `mac` - The mac of the sensor to delete.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the deletion was successful.
    /// * `Err(anyhow::Error)` - If an error occurs during deletion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// service.delete_by_mac("00:1B:44:11:3A:B9").await?;
    /// println!("Sensor deleted.");
    /// # Ok(())
    /// # }
    /// ```

    pub async fn delete_by_mac(&self, mac: &str) -> Result<()> {
        self.repository
            .delete_by_mac(mac)
            .await
            .map_err(map_anyhow_to_sensor_error)
    }

    /// Updates a sensor by its Mac.
    ///
    /// # Arguments
    ///
    /// * `mac` - The mac of the sensor to update.
    /// * `status` - The status of the sensor to update.
    ///
    /// # Returns
    ///
    /// * `Ok<u32>` - Number of sensors updated..
    /// * `Err(anyhow::Error)` - If an error occurs during deletion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::sensors::model::Status;
    /// use db_handler::sensors::model::ProductType;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// let number_of_updated_sensors=service.update_by_mac("00:1B:44:11:3A:B9",Some(Status::Installed),Some(ProductType::ContactSensor)).await?;
    /// if number_of_updated_sensors>0{
    ///     println!("Number of updated sensors: {:?}", number_of_updated_sensors);
    /// } else {
    ///     println!("No sensor updated.");
    /// }
    /// # Ok(())
    /// # }
    /// ```

    pub async fn update_by_mac(
        &self,
        mac: &str,
        status: Option<Status>,
        sensor_type: Option<ProductType>,
    ) -> Result<u32> {
        self.repository
            .update_by_mac(mac, status, sensor_type)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("InProgress") {
                    SensorError::CannotUpdateToInProgress
                } else if msg.contains("At least one field") {
                    SensorError::NoFieldsForUpdate
                } else {
                    SensorError::operation(msg)
                }
            })
    }

    /// Find sensors by a string and paginate the results.
    ///
    /// This function searches for sensors based on the provided search value
    /// and returns a paginated list of results.
    ///
    /// # Arguments
    ///
    /// * `pagination_params` - The parameters for pagination, including:
    ///   - `skip`: The number of items to skip.
    ///   - `nbr`: The number of items per page (`-1` for no limit).
    ///   - `sort_key`: The column to sort by (must be a valid column in the `sensors` table).
    ///   - `sort`: The sorting order (`Asc` or `Desc`).
    ///   - `searched_value`: The optional search term (applies to all string-based columns).
    ///
    /// # Returns
    ///
    /// * `Ok(PaginatedResponse<Sensor>)` - A paginated list of matching sensors.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::sensors::service::SensorService;
    /// use db_handler::db::primary::models::SortOrder;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::sensors::model::{PaginationParams,SensorSortKey};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = SensorService::new(connection);
    /// let pagination_params = PaginationParams {
    ///    skip: 0,
    ///    nbr: -1,  // No limit
    ///    sort_key: SensorSortKey::Id,
    ///    sort: SortOrder::Asc,
    ///    searched_value: None,  // No filter applied
    /// };
    /// let sensors = service.find_by_string(&pagination_params).await?;
    /// println!("{:?}", sensors);
    /// # Ok(())
    /// # }
    /// ```

    pub async fn find_by_string(
        &self,
        pagination_params: &PaginationParams,
    ) -> Result<PaginatedResponse<Sensor>> {
        let pagination_params = CommonPaginationParams {
            skip: pagination_params.skip,
            nbr: pagination_params.nbr,
            sort_key: pagination_params.sort_key.to_string(),
            sort: pagination_params.sort,
            searched_value: pagination_params.searched_value.clone(),
        };
        self.repository
            .find_by_string(&pagination_params)
            .await
            .map_err(map_anyhow_to_sensor_error)
    }
}
#[cfg(test)]
mod tests;
