use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with refresh tokens.
#[derive(Debug, Error)]
pub enum RefreshTokenError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Refresh token not found.
    #[error("Refresh token not found")]
    NotFound,

    /// Refresh token already used.
    #[error("Refresh token already used")]
    AlreadyUsed,

    /// Refresh token expired.
    #[error("Refresh token expired")]
    Expired,

    /// Refresh token already exists.
    #[error("Refresh token already exists")]
    AlreadyExists,
}

/// Result type for refresh token operations.
pub type Result<T> = std::result::Result<T, RefreshTokenError>;

impl RefreshTokenError {
    /// Creates a new not found error for a refresh token ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("RefreshToken".to_string(), id).into()
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a new already used error.
    pub fn already_used() -> Self {
        Self::AlreadyUsed
    }

    /// Creates a new expired error.
    pub fn expired() -> Self {
        Self::Expired
    }
}
