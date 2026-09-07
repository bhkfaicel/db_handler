use std::sync::Arc;

use crate::{
    db::connection::Connection,
    db::primary::{
        error::CommonError, models::PaginatedResponse, repository::PrimaryDbRepository,
        service::Service,
    },
    users::{
        error::{Result, UserError},
        model::{PaginationParams, User},
        password::{
            hash_password_async, password_length_ok, MAX_PASSWORD_BYTES, MIN_PASSWORD_BYTES,
        },
        repository::UserRepository,
    },
};

use super::model::InternalUser;

/// Constraint name SQLite reports for the inline `UNIQUE` on the
/// `users.email` column (initial migration).
const USERS_EMAIL_CONSTRAINT: &str = "users.email";

/// Maps a structured `CommonError` from the repository to the user domain
/// error.
///
/// Unique violations arrive with their constraint name already decoded in a
/// single central place (`CommonError::from_sqlx`); the users table has
/// exactly one unique constraint (`users.email`), so violating it means a
/// duplicate email. Every other error passes through as the transparent
/// `Common` variant, preserving the original database error. `email` is the
/// address the caller was targeting, used only to populate the
/// `EmailAlreadyExists` payload.
fn map_common_error(e: CommonError, email: Option<&str>) -> UserError {
    match e {
        CommonError::UniqueViolation { constraint } if constraint == USERS_EMAIL_CONSTRAINT => {
            UserError::email_already_exists(email.unwrap_or("unknown"))
        }
        other => other.into(),
    }
}

/// Maps a password-hashing failure to the domain error.
fn map_hash_error(e: anyhow::Error) -> UserError {
    // Argon2/PHC failure details never include the hash or the password;
    // surface only the failure message.
    UserError::PasswordHashing(e.to_string())
}

/// Rejects a clear password outside the house policy bounds (8..=128
/// bytes) with the typed input error, before any hashing work.
fn validate_password(password: &str) -> Result<()> {
    if !password_length_ok(password) {
        return Err(UserError::invalid_input(format!(
            "password length must be between {} and {} bytes",
            MIN_PASSWORD_BYTES, MAX_PASSWORD_BYTES
        )));
    }
    Ok(())
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
        // Enforce the house password policy before hashing: reject
        // out-of-range secrets with the typed input error, no Argon2 work.
        validate_password(&password)?;
        // The hash runs on the blocking pool via the async helper: an
        // Argon2id derivation is ~tens of ms of CPU/memory and must never
        // stall the tokio worker inline.
        let password_hash = hash_password_async(password)
            .await
            .map_err(map_hash_error)?;
        let user = InternalUser::new(0, username, email.clone(), Some(password_hash));
        self.repository
            .insert(&user)
            .await
            // The repository reports failures as structured `CommonError`
            // (unique violations included); map them to the domain error.
            .map_err(|e| map_common_error(e, Some(&email)))
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
        self.repository.get_by_id(id).await.map_err(UserError::from)
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
            .map_err(UserError::from)
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
        self.repository
            .find_by_string(pagination_params)
            .await
            .map_err(UserError::from)
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
            .map_err(UserError::from)
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
        // Hashing is domain logic: the clear password is validated and
        // hashed here, in the service, and the repository only persists the
        // hash it is handed.
        let password_hash = match password {
            Some(password) => {
                // Same policy as creation: reject out-of-range reset
                // passwords with the typed input error before hashing.
                validate_password(&password)?;
                // Async helper again: Argon2 on the blocking pool, never
                // inline.
                Some(
                    hash_password_async(password)
                        .await
                        .map_err(map_hash_error)?,
                )
            }
            None => None,
        };
        self.repository
            .update_by_id(id, username, email.clone(), password_hash)
            .await
            // A unique violation here means the target email already
            // belongs to another user; report it with that email.
            .map_err(|e| map_common_error(e, email.as_deref()))
    }
}

#[cfg(test)]
mod tests;
