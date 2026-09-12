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

    /// No user exists for the exact email targeted by a provisioning or
    /// reset operation.
    ///
    /// Deliberately typed (not an opaque operation string) because the
    /// reset/provisioning path NEEDS to tell the operator "this account
    /// does not exist": unlike the login path — which must never reveal
    /// account existence and therefore burns dummy Argon2 work and reports
    /// a uniform `None` — a password reset is an administrative
    /// provisioning surface where the caller is already asking whether the
    /// account exists, so the typed answer leaks nothing the caller does
    /// not already control.
    #[error("No user found for email: {0}")]
    NotFoundByEmail(String),
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
