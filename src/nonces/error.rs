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

    /// Invalid MAC address format. The ledger only accepts six
    /// hexadecimal pairs separated by `:` (case-insensitive); anything
    /// else is refused before any database work happens.
    #[error("Invalid MAC address format: {0}")]
    InvalidMacFormat(String),

    /// The presented nonce value was already accepted before, whatever the
    /// sensor that now presents it: the database `UNIQUE(value)` constraint
    /// rejected the INSERT. This is the anti-replay ledger working as
    /// intended (the value is burned), not an infrastructure failure.
    #[error("Nonce replay detected: value already accepted")]
    ReplayDetected,
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
