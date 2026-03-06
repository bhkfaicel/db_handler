use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with nonces.
#[derive(Debug, Error)]
pub enum NonceError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Invalid nonce value.
    #[error("Invalid nonce value: expected 12 bytes, got {0} bytes")]
    InvalidValue(usize),
}

/// Result type for nonce operations.
pub type Result<T> = std::result::Result<T, NonceError>;

impl NonceError {
    /// Creates a new not found error for a nonce ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("Nonce".to_string(), id).into()
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }
}
