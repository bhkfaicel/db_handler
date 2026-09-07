use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with users.
#[derive(Debug, Error)]
pub enum UserError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Password hashing failed.
    #[error("Failed to hash password: {0}")]
    PasswordHashing(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Email already exists.
    #[error("Email already exists: {0}")]
    EmailAlreadyExists(String),
}

/// Result type for user operations.
pub type Result<T> = std::result::Result<T, UserError>;

impl UserError {
    /// Creates a new invalid input error.
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        UserError::InvalidInput(msg.into())
    }

    /// Creates a new email already exists error.
    pub fn email_already_exists(email: impl Into<String>) -> Self {
        UserError::EmailAlreadyExists(email.into())
    }
}
