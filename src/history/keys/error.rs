use thiserror::Error;

use crate::db::primary::error::CommonError;

/// Errors that can occur when working with historical keys.
#[derive(Debug, Error)]
pub enum HistoricalKeyError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),
}

/// Result type for historical key operations.
pub type Result<T> = std::result::Result<T, HistoricalKeyError>;

impl HistoricalKeyError {
    /// Creates a new not found error for a historical key ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("HistoricalKey".to_string(), id).into()
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }
}
