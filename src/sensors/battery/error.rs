//! Errors for the battery telemetry domain.

use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when recording or reading battery telemetry.
#[derive(Debug, Error)]
pub enum BatteryError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation).
    #[error(transparent)]
    Common(#[from] CommonError),

    /// History write failure. The SQLite latest value is already committed
    /// when this error surfaces, so the local state stays consistent.
    #[error("Failed to write battery history: {0}")]
    HistoryWrite(String),

    /// The reading's timestamp cannot be represented with the nanosecond
    /// precision the InfluxDB line protocol requires.
    #[error("Invalid battery timestamp: {0}")]
    InvalidTimestamp(String),
}

impl BatteryError {
    /// Creates a new history write error.
    pub fn history_write(msg: impl Into<String>) -> Self {
        Self::HistoryWrite(msg.into())
    }

    /// Creates a new invalid timestamp error.
    pub fn invalid_timestamp(msg: impl Into<String>) -> Self {
        Self::InvalidTimestamp(msg.into())
    }
}

/// Result type for battery telemetry operations.
pub type Result<T> = std::result::Result<T, BatteryError>;
