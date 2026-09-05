use sqlx::error::ErrorKind;
use std::sync::Arc;

use crate::{
    common::helpers::controller::hash_password,
    db::connection::Connection,
    db::primary::{
        models::{PaginatedResponse, PaginationParams as CommonPaginationParams},
        repository::PrimaryDbRepository,
        service::{map_sqlx_to_common_error, Service},
    },
    users::{
        error::{Result, UserError},
        model::{PaginationParams, User},
        repository::UserRepository,
    },
};

use super::model::InternalUser;

/// Helper function to convert `sqlx::Error` to `UserError`.
///
/// This function detects UNIQUE constraint violations (e.g., duplicate email) and converts
/// them to `UserError::EmailAlreadyExists`. Other database errors are converted via
/// the centralized `map_sqlx_to_common_error` function.
fn map_sqlx_error(e: sqlx::Error) -> UserError {
    // Check for UNIQUE constraint violation (e.g., duplicate email)
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.kind() == ErrorKind::UniqueViolation {
            // Check if it's an email constraint violation
            let error_message = db_err.message();
            if error_message.contains("users.email")
                || error_message.contains("UNIQUE constraint failed")
            {
                // Try to extract email from error message
                let email = extract_email_from_error(error_message)
                    .unwrap_or_else(|| "unknown".to_string());
                return UserError::email_already_exists(email);
            }
        }
    }
    // For all other errors, use the centralized conversion
    map_sqlx_to_common_error(e).into()
}

/// Helper function to extract email from error message.
fn extract_email_from_error(_error_msg: &str) -> Option<String> {
    // SQLite UNIQUE constraint errors typically don't contain the actual value
    // but we can try to extract it if present
    // For now, return None as SQLite doesn't include the value in the error
    None
}

/// Helper function to convert `anyhow::Error` to `UserError`.
///
/// This function extracts `sqlx::Error` from `anyhow::Error` and uses `map_sqlx_error`
/// for proper conversion. For non-SQLx errors, it converts them to `UserError::operation`.
fn map_anyhow_to_user_error(e: anyhow::Error) -> UserError {
    // Try to extract sqlx::Error from anyhow::Error
    match e.downcast::<sqlx::Error>() {
        Ok(sqlx_err) => {
            // Use the centralized map_sqlx_error function
            map_sqlx_error(sqlx_err)
        }
        Err(e) => {
            // For non-SQLx errors, use operation error
            UserError::operation(e.to_string())
        }
    }
}

pub struct UserService {
    pub(crate) repository: Arc<UserRepository>,
}
impl Service for UserService {
    fn new(connection: Connection) -> Self {
        let repository = UserRepository::new(connection.primary_db_client());
        Self {
            repository: Arc::new(repository),
        }
    }
}
impl UserService {
    /// Inserts a new user into the database.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to the `User` instance to be inserted.
    ///
    /// # Returns
    ///
    /// * `Ok(i32)` - The ID of the newly inserted user.
    /// * `Err(anyhow::Error)` - If an error occurs during insertion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::model::User;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// let id = service.add("username".to_string(),"faycelbhk@gmail.com".to_string(),"clear password".to_string()).await?;
    /// println!("Inserted user with ID: {}", id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add(&self, username: String, email: String, password: String) -> Result<u32> {
        let password_hash = hash_password(&password).map_err(|e| {
            if let Some(be) = e.downcast_ref::<bcrypt::BcryptError>() {
                UserError::PasswordHashing(be.to_string())
            } else {
                UserError::operation(e.to_string())
            }
        })?;
        let user = InternalUser::new(0, username, email, Some(password_hash));
        self.repository
            .insert(&user)
            .await
            .map_err(map_anyhow_to_user_error)
    }

    /// Retrieves a user by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the user to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(user))` - If a user with the specified ID exists.
    /// * `Ok(None)` - If no user with the specified ID is found.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// if let Some(user) = service.get_by_id(1).await? {
    ///     println!("User found: {:?}", user);
    /// } else {
    ///     println!("User not found.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_id(&self, id: u32) -> Result<Option<User>> {
        self.repository
            .get_by_id(id)
            .await
            .map_err(map_anyhow_to_user_error)
    }

    /// Retrieves a user by its Email and clear password.
    ///
    /// # Arguments
    ///
    /// * `email` - The email of the user to retrieve.
    /// * `password` - The non hashed password of the user to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(user))` - If a user with the specified ID exists.
    /// * `Ok(None)` - If no user with the specified ID is found.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// if let Some(user) = service.get_by_email_and_password("faycelbhk@gmail.com".to_string(),"clear password".to_string()).await? {
    ///     println!("User found: {:?}", user);
    /// } else {
    ///     println!("User not found.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_email_and_password(
        &self,
        email: String,
        password: String,
    ) -> Result<Option<User>> {
        self.repository
            .get_by_email_and_password(email, password)
            .await
            .map_err(map_anyhow_to_user_error)
    }

    /// Find users by a string and paginate the results.
    ///
    /// This function searches for users based on the provided search value
    /// and returns a paginated list of results.
    ///
    /// # Arguments
    ///
    /// * `pagination_params` - The parameters for pagination, including:
    ///   - `skip`: The number of items to skip.
    ///   - `nbr`: The number of items per page (`-1` for no limit).
    ///   - `sort_key`: The column to sort by (must be a valid column in the `users` table).
    ///   - `sort`: The sorting order (`Asc` or `Desc`).
    ///   - `searched_value`: The optional search term (applies to all string-based columns).
    ///
    /// # Returns
    ///
    /// * `Ok(PaginatedResponse<User>)` - A paginated list of matching users.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::primary::models::SortOrder;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::users::model::{PaginationParams,UserSortKey};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// let pagination_params = PaginationParams {
    ///    skip: 0,
    ///    nbr: -1,  // No limit
    ///    sort_key: UserSortKey::Id,
    ///    sort: SortOrder::Asc,
    ///    searched_value: None,  // No filter applied
    /// };
    /// let users = service.find_by_string(&pagination_params).await?;
    /// println!("{:?}", users);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_string(
        &self,
        pagination_params: &PaginationParams,
    ) -> Result<PaginatedResponse<User>> {
        let pagination_params = CommonPaginationParams {
            skip: pagination_params.skip,
            nbr: pagination_params.nbr,
            sort_key: pagination_params.sort_key.to_string(),
            sort: pagination_params.sort,
            searched_value: pagination_params.searched_value.clone(),
        };
        self.repository
            .find_by_string(&pagination_params)
            .await
            .map_err(map_anyhow_to_user_error)
    }

    /// Deletes a sensor by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the sensor to delete.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the deletion was successful.
    /// * `Err(anyhow::Error)` - If an error occurs during deletion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// service.delete_by_id(1).await?;
    /// println!("User deleted.");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_by_id(&self, id: u32) -> Result<()> {
        self.repository
            .delete_by_id(id)
            .await
            .map_err(map_anyhow_to_user_error)
    }

    /// Update a user by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the user to update.
    /// * `username` - The username of the user to update.
    /// * `email` - The email of the user to update.
    /// * `password` - The password of the user to update.
    ///
    /// # Returns
    ///
    /// * `Ok<u32>` - Number of users updated.
    /// * `Err(anyhow::Error)` - If an error occurs during retrieval.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = UserService::new(connection.clone());
    /// let number_of_updated_users = service.update_by_id(1,Some("username".to_string()),Some("email".to_string()),Some("clear password".to_string())).await?;
    /// if number_of_updated_users>0{
    ///     println!("Number of updated users: {:?}", number_of_updated_users);
    /// } else {
    ///     println!("No user updated.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_by_id(
        &self,
        id: u32,
        username: Option<String>,
        email: Option<String>,
        password: Option<String>,
    ) -> Result<u32> {
        self.repository
            .update_by_id(id, username, email, password)
            .await
            .map_err(map_anyhow_to_user_error)
    }
}

#[cfg(test)]
mod tests;
