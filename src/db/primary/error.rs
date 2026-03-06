use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommonError {
    /// Generic not found error that carries the entity name and its ID.
    #[error("{0} not found with ID: {1}")]
    NotFound(String, u32),

    /// Database error occurred.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Generic error with context.
    #[error("Operation failed: {0}")]
    Operation(String),
}

pub type Result<T> = std::result::Result<T, CommonError>;
