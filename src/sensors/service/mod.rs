use std::sync::Arc;

use crate::{
    db::{
        connection::Connection,
        primary::{
            error::CommonError, models::PaginatedResponse, repository::PrimaryDbRepository,
            service::Service,
        },
    },
    sensors::{
        error::{Result, SensorError},
        model::{PaginationParams, ProductType, Sensor, Status, TypedSensor},
        repository::SensorRepository,
    },
};

/// Constraint name SQLite reports for the unique index backing the
/// `sensors.mac` column (inline `UNIQUE` in the initial migration).
const SENSORS_MAC_CONSTRAINT: &str = "sensors.mac";

/// Maps a structured `CommonError` from the repository to the sensor
/// domain error.
///
/// Unique violations arrive with their constraint name already decoded in a
/// single central place (`CommonError::from_sqlx`); the sensors table has
/// exactly one unique constraint (`sensors.mac`), so violating it means a
/// duplicate MAC. Every other error passes through as the transparent
/// `Common` variant, preserving the original database error.
fn map_common_error(e: CommonError, mac: &str) -> SensorError {
    match e {
        CommonError::UniqueViolation { constraint } if constraint == SENSORS_MAC_CONSTRAINT => {
            SensorError::duplicate_mac(mac)
        }
        other => other.into(),
    }
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
        // Trust boundary: a sensor MAC must be a well-formed address
        // (six hexadecimal pairs separated by `:`, case-insensitive)
        // because `sensors.mac` roots the foreign-key chain
        // (`keys.mac` -> `nonces.mac`); a hostile value here would flow
        // into every downstream record.
        if !crate::common::helpers::mac::is_valid(mac) {
            return Err(SensorError::invalid_mac(mac));
        }
        let sensor = Sensor::new(0, mac.to_string(), Status::InProgress);
        self.repository
            .insert(&sensor)
            .await
            // The repository reports failures as structured `CommonError`
            // (unique violations included); map them to the domain error.
            .map_err(|e| map_common_error(e, mac))
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
        // The repository already reports errors as `CommonError`, which
        // converts into the transparent `SensorError::Common` via `?`.
        let sensor = self.repository.get_by_id(id).await?;

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
        let sensor = self.repository.get_by_mac(mac).await?;

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
        Ok(self.repository.list_by_status(status).await?)
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
        Ok(self.repository.get_by_mac(mac).await?)
    }

    /// Gets a sensor by its ID, returning the raw sensor.
    ///
    /// This is the by-ID counterpart of `get_by_mac_raw`: unlike `get_by_id`,
    /// which wraps the row in the typed `TypedSensor` enum, it returns the
    /// plain `Sensor` row (including its `sensor_type` column) for callers
    /// that only need the raw data.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the sensor to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Sensor))` - The sensor if found.
    /// * `Ok(None)` - If no sensor with the given ID exists.
    /// * `Err(SensorError)` - If an error occurs during retrieval.
    pub async fn get_by_id_raw(&self, id: u32) -> Result<Option<Sensor>> {
        Ok(self.repository.get_by_id(id).await?)
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
        Ok(self.repository.delete_by_id(id).await?)
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
        Ok(self.repository.delete_by_mac(mac).await?)
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
        // Business rules are validated here, before any SQL is sent: a
        // sensor may only enter the `InProgress` status at creation time,
        // and an update must change at least one field to be meaningful.
        if status == Some(Status::InProgress) {
            return Err(SensorError::CannotUpdateToInProgress);
        }
        if status.is_none() && sensor_type.is_none() {
            return Err(SensorError::NoFieldsForUpdate);
        }
        // The repository is pure data access and reports failures as
        // structured `CommonError`, converted through `?`.
        Ok(self
            .repository
            .update_by_mac(mac, status, sensor_type)
            .await?)
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
        Ok(self.repository.find_by_string(pagination_params).await?)
    }
}
#[cfg(test)]
mod tests;
