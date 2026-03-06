use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with sensor history in time-series databases.
#[derive(Debug, Error)]
pub enum SensorHistoryError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Time-series database connection error.
    #[error("Time-series database connection error: {0}")]
    Connection(String),

    /// Time-series database write error.
    #[error("Failed to write data to time-series database: {0}")]
    Write(String),

    /// Time-series database query error.
    #[error("Failed to query time-series database: {0}")]
    Query(String),

    /// Invalid data format error.
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Result type for sensor history operations.
pub type Result<T> = std::result::Result<T, SensorHistoryError>;

impl SensorHistoryError {
    /// Creates a new not found error for a sensor ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("SensorHistory".to_string(), id).into()
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a new connection error.
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Creates a new write error.
    pub fn write(msg: impl Into<String>) -> Self {
        Self::Write(msg.into())
    }

    /// Creates a new query error.
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Creates a new invalid format error.
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Creates a new configuration error.
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }
}

/// Converts a `SensorHistoryError` to `anyhow::Error`.
///
/// This function converts sensor history errors into `anyhow::Error` format
/// for consistent error handling in service layers that use `anyhow::Result`.
///
/// # Arguments
///
/// * `e` - The sensor history error to convert
///
/// # Returns
///
/// An `anyhow::Error` containing the original error
///
/// # Example
///
/// no_run
/// # use crate::db_handler::history::sensors::error::{SensorHistoryError, map_history_error_to_anyhow};
/// #
/// # let error = SensorHistoryError::write("Failed to write");
/// # let anyhow_error = map_history_error_to_anyhow(error);
///
pub fn map_history_error_to_anyhow(e: SensorHistoryError) -> anyhow::Error {
    anyhow::Error::from(e)
}

#[cfg(test)]
mod tests;
