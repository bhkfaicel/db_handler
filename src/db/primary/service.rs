use crate::{db::connection::Connection, db::primary::error::CommonError};

/// Converts a `sqlx::Error` to a `CommonError`.
///
/// This helper function centralizes the conversion of SQLx errors to common errors.
/// It can be used by all services to convert database errors before converting them
/// to service-specific error types.
///
/// # Example
///
/// ```rust
/// use db_handler::db::primary::service::map_sqlx_to_common_error;
/// use db_handler::refresh_token::error::RefreshTokenError;
///
/// // Convert sqlx::Error → CommonError → RefreshTokenError
/// let sqlx_err = sqlx::Error::PoolClosed;
/// let common_err = map_sqlx_to_common_error(sqlx_err);
/// let refresh_token_err: RefreshTokenError = common_err.into();
/// ```
pub fn map_sqlx_to_common_error(e: sqlx::Error) -> CommonError {
    e.into()
}

/// Base trait for all services in the application.
///
/// This trait defines the common interface for creating and initializing
/// services. All services must implement this trait to ensure a consistent
/// structure across the application.
///
/// # Example
///
/// ```rust
/// // Option 1: Use the prelude (recommended)
/// use db_handler::prelude::*;
/// use db_handler::refresh_token::service::RefreshTokenService;
/// use db_handler::users::service::UserService;
/// use db_handler::db::connection::Connection;
/// use sqlx::SqlitePool;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // All services implement the Service trait
/// let connection = Connection::new().await?;
///
/// // Uniform service creation via the trait
/// let user_service = UserService::new(connection.clone());
/// let refresh_token_service = RefreshTokenService::new(connection.clone());
///
/// // Generic function that accepts any service
/// fn create_service<T: Service>(connection: Connection) -> T {
///     T::new(connection)
/// }
///
/// let user_service2: UserService = create_service(connection);
/// # Ok(())
/// # }
/// ```
///
/// ```rust
/// // Option 2: Explicitly import the trait
/// use db_handler::db::primary::service::Service;
/// use db_handler::users::service::UserService;
/// use db_handler::db::connection::Connection;
/// use sqlx::SqlitePool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = Connection::new().await?;
/// let service = UserService::new(connection);
/// # Ok(())
/// # }
/// ```
pub trait Service: Sized {
    /// Creates a new instance of the service with a Connection.
    ///
    /// # Arguments
    ///
    /// * `connection` - Connection to the database (primary or history)
    ///
    /// # Returns
    ///
    /// A new instance of the service initialized with the provided connection.
    fn new(connection: Connection) -> Self;
}
