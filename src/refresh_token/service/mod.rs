use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::{
    db::connection::Connection,
    db::primary::{error::CommonError, repository::PrimaryDbRepository, service::Service},
    refresh_token::{
        error::{RefreshTokenError, Result},
        model::RefreshToken,
        repository::RefreshTokenRepository,
    },
};

/// Constraint name SQLite reports for the inline `UNIQUE` on the
/// `refresh_tokens.token` column (refresh-tokens migration).
const REFRESH_TOKENS_TOKEN_CONSTRAINT: &str = "refresh_tokens.token";

/// Maps a structured `CommonError` from the repository to the refresh-token
/// domain error.
///
/// Unique violations arrive with their constraint name already decoded in a
/// single central place (`CommonError::from_sqlx`); the refresh_tokens table
/// has exactly one unique constraint (`refresh_tokens.token`), so violating
/// it means a duplicate token. Every other error passes through as the
/// transparent `Common` variant, preserving the original database error.
fn map_common_error(e: CommonError) -> RefreshTokenError {
    match e {
        CommonError::UniqueViolation { constraint }
            if constraint == REFRESH_TOKENS_TOKEN_CONSTRAINT =>
        {
            RefreshTokenError::AlreadyExists
        }
        other => other.into(),
    }
}

pub struct RefreshTokenService {
    pub(crate) repository: Arc<RefreshTokenRepository>,
}
impl Service for RefreshTokenService {
    fn new(connection: Connection) -> Self {
        let repository = RefreshTokenRepository::new(connection.primary_db_client());
        Self {
            repository: Arc::new(repository),
        }
    }
}
impl RefreshTokenService {
    /// Inserts a new refresh token into the database.
    ///
    /// # Arguments
    ///
    /// * `user_id` - user identifier.
    /// * `token` - refresh token value.
    /// * `expire_at` - refresh token expiration time.
    /// * `created_at` - refresh token creation time.
    /// * `is_used` - refresh token used flag.
    ///
    /// # Returns
    ///
    /// * `Ok(RefreshToken)` - The newly inserted refresh token.
    /// * `Err(RefreshTokenError::AlreadyExists)` - If the token already exists.
    /// * `Err(RefreshTokenError::Common(CommonError::Database(_)))` - If a database error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// use chrono::{DateTime, Utc};
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = RefreshTokenService::new(connection.clone());
    ///     let user_service = UserService::new(connection);
    ///     let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    ///     let token = "token";
    ///     let expire_at: DateTime<Utc> = Utc::now();
    ///     let created_at: DateTime<Utc> = Utc::now();
    ///     let is_used = false;
    ///     let refresh_token = service.add(user_id, token, expire_at, created_at).await?;
    ///     println!("Inserted refresh token : {:?}", refresh_token);
    ///     Ok(())
    ///  }
    /// ```
    pub async fn add(
        &self,
        user_id: u32,
        token: &str,
        expire_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Result<RefreshToken> {
        let mut refresh_token = RefreshToken {
            id: 0,
            user_id,
            token: token.to_string(),
            expire_at,
            created_at,
            is_used: false,
        };

        // Retention piggyback: purge this user's expired and used tokens
        // before issuing the new one. Issuance is a guaranteed path, so
        // per-user growth stays bounded to the user's live tokens, and the
        // purge rides the existing `idx_refresh_tokens_user_id` index.
        // Tokens of users who never log in again are not reached here: the
        // public `cleanup_expired` hook covers that residual (documented as
        // caller-dependent, not a resolution).
        self.repository
            .delete_expired_for_user(user_id, Utc::now())
            .await
            .map_err(map_common_error)?;

        let id = self
            .repository
            .insert(&refresh_token)
            .await
            .map_err(map_common_error)?;
        refresh_token.id = id;
        Ok(refresh_token)
    }

    /// Deletes every expired refresh token across all users.
    ///
    /// Caller-dependent boot-time sweep: token issuance only purges the
    /// logging-in user's rows, so tokens of users who never log in again
    /// linger until an operator (or the application boot sequence)
    /// invokes this method. The library never schedules background work
    /// on its own; calling this hook is the caller's decision.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the sweep was successful.
    /// * `Err(RefreshTokenError::Common(CommonError::Database(_)))` - If a database error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = RefreshTokenService::new(connection);
    ///     service.cleanup_expired().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn cleanup_expired(&self) -> Result<()> {
        self.repository
            .delete_expired(Utc::now())
            .await
            .map_err(map_common_error)?;
        Ok(())
    }

    /// Deletes all refresh tokens and all associated tokens for the user.
    ///
    /// # Arguments
    /// * `user_id` - user identifier.
    /// # Returns
    /// * `Ok(())` - If the deletion was successful.
    /// * `Err(RefreshTokenError::Common(CommonError::Database(_)))` - If a database error occurs.
    /// # Example
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::refresh_token::error::RefreshTokenError;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::users::service::UserService;
    /// use chrono::{DateTime, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// let token = "token";
    /// let expire_at: DateTime<Utc> = Utc::now();
    /// let created_at: DateTime<Utc> = Utc::now();
    /// let refresh_token = service.add(user_id, token, expire_at, created_at).await?;
    /// service.delete_by_user_id(user_id).await?;
    /// // Token was deleted, so it's not found
    /// let result = service.is_valid(user_id, token).await;
    /// assert!(matches!(result, Err(RefreshTokenError::NotFound)));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_by_user_id(&self, user_id: u32) -> Result<()> {
        self.repository
            .delete_by_user_id(user_id)
            .await
            .map_err(map_common_error)?;
        Ok(())
    }

    /// Checks if a refresh token is valid.
    ///
    /// # Behavior
    ///
    /// - If the token is not found, returns `RefreshTokenError::NotFound`.
    /// - If the token is expired, returns `RefreshTokenError::Expired`.
    /// - If the token is already used, **all tokens for the user are deleted** and returns `RefreshTokenError::AlreadyUsed`.
    /// - If the token is valid, returns `Ok(true)`.
    ///
    /// # Arguments
    /// * `user_id` - user identifier.
    /// * `token` - refresh token value.
    /// # Returns
    /// * `Ok(true)` - If the refresh token is valid.
    /// * `Err(RefreshTokenError::NotFound)` - If the token is not found.
    /// * `Err(RefreshTokenError::Expired)` - If the token has expired.
    /// * `Err(RefreshTokenError::AlreadyUsed)` - If the token has already been used (all user tokens are deleted).
    /// * `Err(RefreshTokenError::Common(CommonError::Database(_)))` - If a database error occurs.
    /// # Example
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::refresh_token::error::RefreshTokenError;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::users::service::UserService;
    /// use chrono::{DateTime, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// let expire_at: DateTime<Utc> = Utc::now();
    /// let created_at: DateTime<Utc> = Utc::now();
    /// let token = "token";
    /// service.add(user_id, token, expire_at, created_at).await?;
    /// let token2 = "token2";
    /// let refresh_token = service.add(user_id, token, expire_at, created_at).await?;
    /// // Token exists and is valid
    /// let result = service.is_valid(user_id, token).await?;
    /// assert_eq!(result, true);
    /// // Token doesn't exist
    /// let result2 = service.is_valid(user_id, token2).await;
    /// assert!(matches!(result2, Err(RefreshTokenError::NotFound)));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_valid(&self, user_id: u32, token: &str) -> Result<bool> {
        let result = self
            .repository
            .get_by_user_id_and_token(user_id, token)
            .await
            .map_err(map_common_error)?;
        if let Some(record) = result {
            if record.expire_at < chrono::Utc::now() {
                Err(RefreshTokenError::Expired)
            } else if record.is_used {
                self.repository
                    .delete_by_user_id(user_id)
                    .await
                    .map_err(map_common_error)?;
                Err(RefreshTokenError::AlreadyUsed)
            } else {
                Ok(true)
            }
        } else {
            Err(RefreshTokenError::NotFound)
        }
    }

    /// Updates the used flag of a refresh token.
    ///
    /// # Arguments
    /// * `token` - refresh token value.
    /// # Returns
    /// * `Ok(())` - If the update was successful.
    /// * `Err(RefreshTokenError)` - If an error occurs during update.
    /// # Example
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::refresh_token::error::RefreshTokenError;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::users::service::UserService;
    /// use chrono::{DateTime, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// let expire_at: DateTime<Utc> = Utc::now();
    /// let created_at: DateTime<Utc> = Utc::now();
    /// let token = "token";
    /// service.add(user_id, token, expire_at, created_at).await?;
    /// service.update_flag_by_token(token).await?;
    /// // Token is now marked as used, so validation will delete all user tokens and return AlreadyUsed
    /// let result = service.is_valid(user_id, token).await;
    /// assert!(matches!(result, Err(RefreshTokenError::AlreadyUsed)));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_flag_by_token(&self, token: &str) -> Result<()> {
        self.repository
            .update_flag_by_token(token)
            .await
            .map_err(map_common_error)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
