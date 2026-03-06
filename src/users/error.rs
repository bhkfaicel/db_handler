use crate::db::primary::error::CommonError;
use thiserror::Error;

/// Errors that can occur when working with users.
#[derive(Debug, Error)]
pub enum UserError {
    /// Common reusable errors (e.g., generic NotFound, Database, Operation)
    #[error(transparent)]
    Common(#[from] CommonError),

    /// User not found for the specified email.
    #[error("User not found with email: {0}")]
    NotFoundByEmail(String),

    /// Invalid credentials provided.
    #[error("Invalid credentials for user: {0}")]
    InvalidCredentials(String),

    /// Password hashing failed.
    #[error("Failed to hash password: {0}")]
    PasswordHashing(String),

    /// Password verification failed.
    #[error("Failed to verify password: {0}")]
    PasswordVerification(String),

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
    /// Creates a new not found error for a user ID.
    pub fn not_found(id: u32) -> Self {
        CommonError::NotFound("User".to_string(), id).into()
    }

    /// Creates a new not found error for an email.
    pub fn not_found_by_email(email: impl Into<String>) -> Self {
        UserError::NotFoundByEmail(email.into())
    }

    /// Creates a new invalid credentials error.
    pub fn invalid_credentials(email: impl Into<String>) -> Self {
        UserError::InvalidCredentials(email.into())
    }

    /// Creates a new operation error with context.
    pub fn operation(msg: impl Into<String>) -> Self {
        CommonError::Operation(msg.into()).into()
    }

    /// Creates a new invalid input error.
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        UserError::InvalidInput(msg.into())
    }

    /// Creates a new email already exists error.
    pub fn email_already_exists(email: impl Into<String>) -> Self {
        UserError::EmailAlreadyExists(email.into())
    }
}

impl From<bcrypt::BcryptError> for UserError {
    fn from(err: bcrypt::BcryptError) -> Self {
        UserError::PasswordHashing(err.to_string())
    }
}
