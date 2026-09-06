use crate::{db::connection::Connection, db::primary::error::CommonError};

/// Converts a `sqlx::Error` to a `CommonError`.
///
/// This helper function is the crate's single conversion point from SQLx
/// errors to common errors. It delegates to `CommonError::from_sqlx`, so
/// unique violations are decoded into the structured
/// `CommonError::UniqueViolation` variant exactly once, here, and services
/// never need to sniff error messages themselves.
///
/// # Example
///
/// ```rust
/// use db_handler::db::primary::service::map_sqlx_to_common_error;
/// use db_handler::db::primary::error::CommonError;
///
/// // sqlx::Error -> CommonError; mapping the result onto a domain error is
/// // the caller's business.
/// let common_err = map_sqlx_to_common_error(sqlx::Error::PoolClosed);
/// assert!(matches!(common_err, CommonError::Database(_)));
/// ```
pub fn map_sqlx_to_common_error(e: sqlx::Error) -> CommonError {
    CommonError::from_sqlx(e)
}

/// Base trait for all services in the application.
///
/// This trait defines the common interface for creating and initializing
/// services. All services must implement this trait to ensure a consistent
/// structure across the application.
///
/// # Example
///
/// The examples below use `UserService`, which only exists under the
/// `users` feature, so each example is compiled and run only when that
/// feature is enabled (the `#[cfg]` guards keep the doc tests green in a
/// core-only build too).
///
/// ```rust
/// # #[cfg(feature = "users")]
/// # mod prelude_example {
/// // Option 1: use the prelude (recommended)
/// use db_handler::prelude::*;
/// use db_handler::users::service::UserService;
/// use db_handler::db::connection::Connection;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // All services implement the Service trait
/// let connection = Connection::new().await?;
///
/// // Uniform service creation via the trait
/// let user_service = UserService::new(connection.clone());
///
/// // Generic function that accepts any service
/// fn create_service<T: Service>(connection: Connection) -> T {
///     T::new(connection)
/// }
///
/// let user_service2: UserService = create_service(connection);
/// # let _ = (user_service, user_service2);
/// # Ok(())
/// # }
/// # }
/// ```
///
/// ```rust
/// # #[cfg(feature = "users")]
/// # mod explicit_example {
/// // Option 2: explicitly import the trait
/// use db_handler::db::primary::service::Service;
/// use db_handler::users::service::UserService;
/// use db_handler::db::connection::Connection;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = Connection::new().await?;
/// let service = UserService::new(connection);
/// # let _ = service;
/// # Ok(())
/// # }
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
