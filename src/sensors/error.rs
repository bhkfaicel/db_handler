use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with sensors.
#[derive(Debug, Error)]
pub enum SensorError {
    /// Common reusable errors (e.g., generic NotFound)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Sensor not found for the specified MAC address.
    #[error("Sensor not found with MAC address: {0}")]
    NotFoundByMac(String),

    /// Invalid MAC address format or empty MAC.
    #[error("Invalid MAC address: {0}")]
    InvalidMac(String),

    /// Invalid sensor status for the operation.
    #[error("Invalid sensor status: {0}")]
    InvalidStatus(String),

    /// Status cannot be updated to InProgress.
    #[error("Cannot update sensor status to 'InProgress': this status is only set during sensor creation")]
    CannotUpdateToInProgress,

    /// No fields provided for update.
    #[error("At least one field (status or sensor_type) must be provided for update")]
    NoFieldsForUpdate,

    /// Duplicate MAC address - sensor with this MAC already exists.
    #[error("A sensor with MAC address '{0}' already exists")]
    DuplicateMac(String),
}

/// Result type for sensor operations.
pub type Result<T> = std::result::Result<T, SensorError>;

impl SensorError {
    /// Creates a new not found error for a sensor ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("Sensor".to_string(), id).into()
    }

    /// Creates a new not found error for a MAC address.
    pub fn not_found_by_mac(mac: impl Into<String>) -> Self {
        SensorError::NotFoundByMac(mac.into())
    }

    /// Creates a new invalid MAC error.
    pub fn invalid_mac(mac: impl Into<String>) -> Self {
        SensorError::InvalidMac(mac.into())
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a new duplicate MAC error.
    pub fn duplicate_mac(mac: impl Into<String>) -> Self {
        SensorError::DuplicateMac(mac.into())
    }
}
