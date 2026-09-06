#[cfg(feature = "utoipaDoc")]
use utoipa::ToSchema;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{drives::columns::HasColumns, sensors::contact::model::ContactSensor};
use db_derive::HasColumns;

/// Data for recording sensor history.
///
/// This enum represents different types of history data depending on the sensor type.
#[derive(Debug, Clone)]
pub enum HistoryData {
    /// History data for contact sensors
    Contact {
        /// Whether the sensor detects an open state
        is_open: bool,
        /// Optional vibration count measurement
        vibration_count: Option<crate::history::sensors::model::fields::VibrationCount>,
    },
}
/// Represents the status of a sensor.
#[derive(Debug, sqlx::Type, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
pub enum Status {
    /// The sensor is currently being installed.
    InProgress,
    /// The sensor has been successfully installed.
    Installed,
    /// The sensor is undergoing an update.
    OnUpdate,
}

impl std::str::FromStr for Status {
    type Err = ();

    /// Converts a string slice to a `Status`.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice representing the status.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::sensors::model::Status;
    /// use std::str::FromStr;
    ///
    /// let status = Status::from_str("Installed").unwrap();
    /// assert_eq!(status, Status::Installed);
    /// ```
    fn from_str(input: &str) -> Result<Status, Self::Err> {
        match input {
            "InProgress" => Ok(Status::InProgress),
            "Installed" => Ok(Status::Installed),
            "OnUpdate" => Ok(Status::OnUpdate),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Status {
    /// Formats the `Status` as a string.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::sensors::model::Status;
    ///
    /// let status = Status::InProgress;
    /// assert_eq!(status.to_string(), "InProgress");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::InProgress => write!(f, "InProgress"),
            Status::Installed => write!(f, "Installed"),
            Status::OnUpdate => write!(f, "OnUpdate"),
        }
    }
}

/// Represents the type of a sensor.
#[derive(Debug, sqlx::Type, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
pub enum ProductType {
    /// Detects open/close status of things like windows or doors.
    ContactSensor,
}

impl std::str::FromStr for ProductType {
    type Err = ();

    /// Converts a string slice to a `ProductType`.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice representing the product type.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::sensors::model::ProductType;
    /// use std::str::FromStr;
    ///
    /// let sensor_type = ProductType::from_str("ContactSensor").unwrap();
    /// assert_eq!(sensor_type, ProductType::ContactSensor);
    /// ```
    fn from_str(input: &str) -> Result<crate::sensors::model::ProductType, Self::Err> {
        match input {
            "ContactSensor" => Ok(ProductType::ContactSensor),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ProductType {
    /// Formats the `ProductType` as a string.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::sensors::model::ProductType;
    ///
    /// let sensor_type = ProductType::ContactSensor;
    /// assert_eq!(sensor_type.to_string(), "ContactSensor");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductType::ContactSensor => write!(f, "ContactSensor"),
        }
    }
}

/// Represents a sensor with an ID, MAC address, and status.
#[derive(Debug, Clone, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
pub struct Sensor {
    /// Unique identifier for the sensor.
    pub id: u32,
    /// MAC address of the sensor.
    pub mac: String,
    /// Current status of the sensor.
    pub status: Status,
    pub sensor_type: Option<ProductType>,
}

impl Sensor {
    /// Creates a new `Sensor` instance.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the sensor.
    /// * `mac` - MAC address of the sensor.
    /// * `status` - Initial status of the sensor.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::sensors::model::{Sensor, Status};
    ///
    /// let sensor = Sensor::new(1, "00:1B:44:11:3A:B7".to_string(), Status::InProgress);
    /// assert_eq!(sensor.id, 1);
    /// assert_eq!(sensor.mac, "00:1B:44:11:3A:B7");
    /// assert_eq!(sensor.status, Status::InProgress);
    /// ```
    pub fn new(id: u32, mac: String, status: Status) -> Self {
        Sensor {
            id,
            mac,
            status,
            sensor_type: None,
        }
    }
}

/// Trait for common operations on typed sensors.
///
/// All typed sensor implementations should provide access to basic sensor information
/// like ID and MAC address. This allows generic handling of different sensor types.
pub trait TypedSensorTrait {
    /// Gets the sensor ID.
    fn id(&self) -> u32;

    /// Gets the sensor MAC address.
    fn mac(&self) -> &str;

    /// Gets the sensor status.
    fn status(&self) -> &Status;
}

/// Represents a sensor with its specific type.
///
/// This enum allows returning the correct sensor type from methods like `get_by_mac`,
/// making it easier to use type-specific methods like `set_history`.
#[derive(Debug, Clone)]
pub enum TypedSensor {
    /// A sensor without a defined type (sensor_type is None)
    Unknown(Sensor),
    ContactSensor(ContactSensor),
}

impl TypedSensor {
    /// Creates a `TypedSensor` from a `Sensor` based on its type.
    ///
    /// The conversion matches exhaustively on `ProductType`, so adding a new
    /// sensor type is a compile error until a matching helper function (and
    /// the typed model behind it) exists.
    ///
    /// # Arguments
    ///
    /// * `sensor` - The sensor to convert
    ///
    /// # Returns
    ///
    /// * `TypedSensor` - The typed sensor (Unknown if sensor_type is None)
    pub fn from_sensor(sensor: Sensor) -> Self {
        match sensor.sensor_type {
            Some(ProductType::ContactSensor) => Self::from_contact_sensor(sensor),
            None => TypedSensor::Unknown(sensor),
        }
    }

    /// Helper function to create a `ContactSensor` from a `Sensor`.
    ///
    /// Wraps the raw sensor in its typed representation; `ContactSensor`
    /// implements `TypedSensorTrait`, which the `TypedSensor` accessors
    /// delegate to.
    ///
    /// # Arguments
    ///
    /// * `sensor` - The sensor to convert
    ///
    /// # Returns
    ///
    /// * `TypedSensor::ContactSensor` - The typed contact sensor
    fn from_contact_sensor(sensor: Sensor) -> Self {
        let contact = ContactSensor::new(
            sensor, None, // is_open not loaded yet
            None, // updated_at not loaded yet
        );

        TypedSensor::ContactSensor(contact)
    }

    /// Gets the underlying sensor ID.
    ///
    /// This method delegates to the trait implementation for each sensor type.
    pub fn id(&self) -> u32 {
        match self {
            TypedSensor::Unknown(s) => s.id,
            TypedSensor::ContactSensor(s) => s.id(),
        }
    }

    /// Gets the underlying sensor MAC address.
    ///
    /// This method delegates to the trait implementation for each sensor type.
    pub fn mac(&self) -> &str {
        match self {
            TypedSensor::Unknown(s) => &s.mac,
            TypedSensor::ContactSensor(s) => s.mac(),
        }
    }

    /// Gets the underlying sensor status.
    ///
    /// This method delegates to the trait implementation for each sensor type.
    pub fn status(&self) -> &Status {
        match self {
            TypedSensor::Unknown(s) => &s.status,
            TypedSensor::ContactSensor(s) => s.status(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SensorSortKey {
    Id,
    Mac,
    Status,
}

impl AsRef<str> for SensorSortKey {
    fn as_ref(&self) -> &str {
        match self {
            SensorSortKey::Id => "id",
            SensorSortKey::Mac => "mac",
            SensorSortKey::Status => "status",
        }
    }
}

impl std::fmt::Display for SensorSortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorSortKey::Id => write!(f, "id"),
            SensorSortKey::Mac => write!(f, "mac"),
            SensorSortKey::Status => write!(f, "status"),
        }
    }
}

/// Pagination parameters for sensor queries: the crate-wide generic
/// pagination model instantiated with the typed sensor sort key, so the
/// sort column comes from `SensorSortKey` itself instead of a free-form
/// string.
pub type PaginationParams = crate::db::primary::models::PaginationParams<SensorSortKey>;

#[cfg(test)]
pub mod tests;
