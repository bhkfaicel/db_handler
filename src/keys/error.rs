use crate::{db::primary::error::CommonError, history::keys::error::HistoricalKeyError};
use thiserror::Error;

/// Errors that can occur when working with keys.
#[derive(Debug, Error)]
pub enum KeyError {
    /// Common reusable errors (e.g., generic Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),
    /// Key not found for the specified MAC address.
    #[error("Key not found for MAC address: {0}")]
    NotFound(String),

    /// Invalid MAC address format.
    #[error("Invalid MAC address format: {0}")]
    InvalidMacFormat(String),

    /// Key already exists in historical keys (cannot be reused).
    #[error("Key already exists in historical keys and cannot be reused")]
    AlreadyInHistory,

    /// Error from historical keys service.
    #[error("Historical key error: {0}")]
    HistoricalKey(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Result type for key operations.
pub type Result<T> = std::result::Result<T, KeyError>;

impl KeyError {
    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a not found error for a MAC address.
    pub fn not_found(mac: impl Into<String>) -> Self {
        KeyError::NotFound(mac.into())
    }

    /// Creates an invalid MAC format error.
    pub fn invalid_mac(mac: impl Into<String>) -> Self {
        KeyError::InvalidMacFormat(mac.into())
    }
}

impl From<HistoricalKeyError> for KeyError {
    fn from(err: HistoricalKeyError) -> Self {
        KeyError::HistoricalKey(Box::new(err))
    }
}
