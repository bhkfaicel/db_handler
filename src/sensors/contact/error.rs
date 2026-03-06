use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with contact sensors.
#[derive(Debug, Error)]
pub enum ContactSensorError {
    /// Common reusable errors (e.g., generic Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),
    /// Contact sensor not found for the specified ID/MAC.
    #[error("Contact sensor not found for ID/MAC: {0}")]
    NotFound(String),
}

/// Result type for contact sensor operations.
pub type Result<T> = std::result::Result<T, ContactSensorError>;

impl ContactSensorError {
    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a not found error for a ID/MAC.
    pub fn not_found(mac: impl Into<String>) -> Self {
        ContactSensorError::NotFound(mac.into())
    }
}
