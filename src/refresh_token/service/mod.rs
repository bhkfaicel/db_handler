use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::{
    db::connection::Connection,
    db::primary::{error::CommonError, repository::PrimaryDbRepository, service::Service},
    refresh_token::{
        error::{RefreshTokenError, Result, RotationOutcome},
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

/// Largest presented session token accepted, in bytes.
///
/// Sessions are opaque caller-generated values (256-bit CSPRNG output is
/// 43 base64url characters; even large signed structures stay far below
/// this ceiling), so anything beyond it is either abuse or a bug and is
/// rejected BEFORE any SQL runs. Deliberately generous: it bounds the
/// value hashed per call, not the format (the library never parses
/// session tokens).
const MAX_SESSION_TOKEN_BYTES: usize = 4096;

/// Rejects empty or oversized presented session values BEFORE SQL.
///
/// An empty value would otherwise store/look up the digest of the empty
/// string, and an oversized value would let callers burn arbitrary hashing
/// work; both are caller bugs, rejected with the typed operation error
/// before the repository is reached. Raw values are never echoed back in
/// the error (fixed message only).
fn validate_presented_session_token(token: &str) -> Result<()> {
    if token.is_empty() || token.len() > MAX_SESSION_TOKEN_BYTES {
        return Err(RefreshTokenError::operation(format!(
            "session token must be non-empty and at most {MAX_SESSION_TOKEN_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Rejects an inverted session validity window BEFORE SQL.
///
/// A session whose expiry is not strictly after its creation would be born
/// already expired (the validation predicate requires `expire_at > now`);
/// refusing the window at insert time keeps the failure at the store call
/// instead of a silently useless row.
fn validate_session_window(expire_at: DateTime<Utc>, created_at: DateTime<Utc>) -> Result<()> {
    if expire_at <= created_at {
        return Err(RefreshTokenError::operation(
            "session expire_at must be strictly after created_at",
        ));
    }
    Ok(())
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
    /// # Token entropy contract
    ///
    /// `token` MUST carry at least 256 bits of cryptographic
    /// unpredictability: an opaque CSPRNG value, or a signature under a
    /// signing key of 256 bits or more, with unique per-token claims. Only
    /// the SHA-256 digest is stored at rest, so this unpredictability is
    /// the sole protection against digest guessing or precomputation.
    /// Caller qualification (the backend's `SECRET_KEY` entropy and token
    /// uniqueness) is a documented coordination item, UNVERIFIED here.
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
    ///     println!("Inserted refresh token for user {}", refresh_token.user_id);
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

        // Retention piggyback: purge this user's EXPIRED tokens before
        // issuing the new one (expired-only policy). Already-used tokens are
        // deliberately retained until their natural expiry so a replayed
        // consumed token keeps classifying as reuse for the token's full
        // lifetime; per-user growth stays bounded by the expiry window, and
        // the purge rides the existing `idx_refresh_tokens_user_id` index.
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
    /// use db_handler::refresh_token::error::RotationOutcome;
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
    /// let refresh_token = service.add(user_id, token, expire_at, created_at).await?;
    /// service.delete_by_user_id(user_id).await?;
    /// // Token was deleted, so rotation classifies it as not found
    /// let result = service.rotate(user_id, token, "replacement", expire_at, created_at).await;
    /// assert!(matches!(result, Ok(RotationOutcome::NotFound)));
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

    /// Atomically consumes a refresh token and replaces it with a new one.
    ///
    /// Consumption and replacement happen in ONE database transaction
    /// (SQLite serializes writers, so concurrent rotations of the same token
    /// are totally ordered: the first commit consumes the token, every later
    /// commit classifies as reuse and deletes all the user's tokens).
    ///
    /// # Token entropy contract
    ///
    /// `old_token` and `new_token` MUST each carry at least 256 bits of
    /// cryptographic unpredictability (opaque CSPRNG value, or signature
    /// under a >= 256-bit key with unique claims): only their SHA-256
    /// digests are stored and matched at rest. Caller qualification (the
    /// backend's `SECRET_KEY` entropy and token uniqueness) is a documented
    /// coordination item, UNVERIFIED here.
    ///
    /// # Retention contract
    ///
    /// Used tokens are retained until `expire_at` (the per-user purge is
    /// expired-only): a replayed consumed token keeps classifying as
    /// [`RotationOutcome::ReuseDetected`] for the token's full lifetime, and
    /// per-user storage stays bounded by the expiry window rather than by
    /// the number of rotations.
    ///
    /// # Behavior
    ///
    /// - If no token matches `(user_id, old_token)`, returns
    ///   `Ok(RotationOutcome::NotFound)`.
    /// - If the token has expired, returns `Ok(RotationOutcome::Expired)`
    ///   and the token is left untouched.
    /// - If the token was already consumed (replay), **all tokens for the
    ///   user are deleted** and `Ok(RotationOutcome::ReuseDetected)` is
    ///   returned.
    /// - Otherwise the token is consumed and the replacement inserted in the
    ///   same transaction; `Ok(RotationOutcome::Rotated)` carries the
    ///   inserted replacement.
    ///
    /// # Arguments
    /// * `user_id` - user identifier.
    /// * `old_token` - refresh token value to consume.
    /// * `new_token` - replacement refresh token value to insert.
    /// * `expire_at` - replacement token expiration time.
    /// * `created_at` - replacement token creation time.
    ///
    /// # Returns
    /// * `Ok(RotationOutcome::Rotated)` - the token was consumed and its replacement inserted.
    /// * `Ok(RotationOutcome::NotFound)` - no token matches (user_id, old_token).
    /// * `Ok(RotationOutcome::Expired)` - the token has expired (left unconsumed).
    /// * `Ok(RotationOutcome::ReuseDetected)` - the token was already consumed (replay); all user tokens were deleted.
    /// * `Err(RefreshTokenError::AlreadyExists)` - the replacement token value already exists; the transaction rolled back, so nothing was consumed.
    /// * `Err(RefreshTokenError::Common(CommonError::Database(_)))` - If a database error occurs.
    ///
    /// # Example
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::refresh_token::error::RotationOutcome;
    /// use db_handler::db::connection::Connection;
    /// use db_handler::users::service::UserService;
    /// use chrono::{DateTime, Duration, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// let expire_at: DateTime<Utc> = Utc::now() + Duration::hours(1);
    /// let created_at: DateTime<Utc> = Utc::now();
    /// service.add(user_id, "token", expire_at, created_at).await?;
    /// // First presentation: the token is consumed and replaced in one
    /// // transaction.
    /// let outcome = service.rotate(user_id, "token", "rotated", expire_at, created_at).await?;
    /// assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    /// // Replaying the consumed token burns every token of the user.
    /// let outcome = service.rotate(user_id, "token", "again", expire_at, created_at).await?;
    /// assert_eq!(outcome, RotationOutcome::ReuseDetected);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rotate(
        &self,
        user_id: u32,
        old_token: &str,
        new_token: &str,
        expire_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Result<RotationOutcome> {
        let new_refresh_token = RefreshToken {
            id: 0,
            user_id,
            token: new_token.to_string(),
            expire_at,
            created_at,
            is_used: false,
        };
        self.repository
            .rotate(user_id, old_token, &new_refresh_token, Utc::now())
            .await
            .map_err(map_common_error)
    }

    /// Stores a gRPC session for `(user_id, raw_token, expire_at,
    /// created_at)`.
    ///
    /// Sessions live in the same `refresh_tokens` table as REST refresh
    /// tokens but are cryptographically domain-separated at the storage
    /// boundary: a session is stored under the tagged digest
    /// `grpc-session:<sha256(raw)>` while refresh tokens stay bare 64-hex
    /// digests, so the same raw value can exist in both domains as two
    /// distinct rows and no refresh path can consume, purge or revoke a
    /// session (nor the reverse). No migration was needed: the tagged value
    /// fits the existing `TEXT NOT NULL UNIQUE` column.
    ///
    /// # Token entropy contract
    ///
    /// `token` MUST carry at least 256 bits of cryptographic
    /// unpredictability from the caller's CSPRNG (the library never
    /// generates tokens). Only the tagged SHA-256 digest is stored at
    /// rest, so that unpredictability is the sole protection against
    /// digest guessing or precomputation. The presented value is
    /// validated BEFORE SQL: empty or oversized values (over 4096 bytes)
    /// are rejected, as is a window where `expire_at` is not strictly
    /// after `created_at`.
    ///
    /// # Errors
    ///
    /// * `RefreshTokenError::AlreadyExists` - the same raw token is
    ///   already stored as a session for any user: the prior row is kept
    ///   untouched (no overwrite, no expiry extension).
    /// * `RefreshTokenError::Common(CommonError::Operation(_))` - empty
    ///   or oversized token, or `expire_at <= created_at`.
    /// * `RefreshTokenError::Common(CommonError::Database(_))` - database
    ///   failure (including an unknown `user_id`, via the foreign key).
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// use chrono::{Duration, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// // The caller generates the opaque value (>= 256 bits of entropy).
    /// service.add_session(user_id, "opaque-session-value", Utc::now() + Duration::hours(12), Utc::now()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_session(
        &self,
        user_id: u32,
        token: &str,
        expire_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        validate_presented_session_token(token)?;
        validate_session_window(expire_at, created_at)?;
        self.repository
            .insert_session(user_id, token, expire_at, created_at)
            .await
            .map_err(map_common_error)
    }

    /// Non-consumingly validates a presented session token and returns
    /// the owning user ID.
    ///
    /// The whole decision lives in one SQL predicate: the row must match
    /// the tagged digest of the presented value AND be unrevoked AND be
    /// strictly unexpired. Validation never mutates the row, so it can
    /// repeat until revocation or expiry — this is what makes it suitable
    /// for per-request authentication, unlike the consuming REST rotation.
    /// Unknown, expired and revoked sessions all return `Ok(None)`
    /// (fail closed) and are indistinguishable.
    ///
    /// # Errors
    ///
    /// * `RefreshTokenError::Common(CommonError::Operation(_))` - empty
    ///   or oversized presented value.
    /// * `RefreshTokenError::Common(CommonError::Database(_))` - database
    ///   failure.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// use chrono::{Duration, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// service.add_session(user_id, "opaque-session-value", Utc::now() + Duration::hours(12), Utc::now()).await?;
    /// // Non-consuming: repeatable until revocation or expiry.
    /// assert_eq!(service.validate_session("opaque-session-value").await?, Some(user_id));
    /// assert_eq!(service.validate_session("opaque-session-value").await?, Some(user_id));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_session(&self, token: &str) -> Result<Option<u32>> {
        validate_presented_session_token(token)?;
        self.repository
            .session_user_id(token, Utc::now())
            .await
            .map_err(map_common_error)
    }

    /// Revokes the presented session in one atomic SQL statement.
    ///
    /// Idempotent by design: an unknown, already-revoked or expired value
    /// matches zero rows and still succeeds, so logout never errors on a
    /// stale token. Only session-domain rows are reachable; REST refresh
    /// tokens of the same user are untouched. The raw presented value is
    /// hashed before it reaches SQL and never stored or returned.
    ///
    /// # Errors
    ///
    /// * `RefreshTokenError::Common(CommonError::Operation(_))` - empty
    ///   or oversized presented value.
    /// * `RefreshTokenError::Common(CommonError::Database(_))` - database
    ///   failure.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// use chrono::{Duration, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// service.add_session(user_id, "opaque-session-value", Utc::now() + Duration::hours(12), Utc::now()).await?;
    /// service.revoke_session("opaque-session-value").await?;
    /// // Revoked: validation fails closed.
    /// assert_eq!(service.validate_session("opaque-session-value").await?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revoke_session(&self, token: &str) -> Result<()> {
        validate_presented_session_token(token)?;
        self.repository
            .revoke_session(token)
            .await
            .map_err(map_common_error)
    }

    /// Revokes every session of one user in one atomic SQL statement
    /// (logout-everywhere).
    ///
    /// Only session-domain rows of `user_id` flip to revoked; REST refresh
    /// tokens of the same user stay untouched, so a gRPC logout-all never
    /// breaks an in-flight REST refresh rotation.
    ///
    /// # Errors
    ///
    /// * `RefreshTokenError::Common(CommonError::Database(_))` - database
    ///   failure.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::users::service::UserService;
    /// use db_handler::db::connection::Connection;
    /// use chrono::{Duration, Utc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection.clone());
    /// let user_service = UserService::new(connection);
    /// let user_id = user_service.add("username".to_string(), "email@example.com".to_string(), "password".to_string()).await?;
    /// service.add_session(user_id, "first", Utc::now() + Duration::hours(12), Utc::now()).await?;
    /// service.add_session(user_id, "second", Utc::now() + Duration::hours(12), Utc::now()).await?;
    /// service.revoke_all_sessions(user_id).await?;
    /// assert_eq!(service.validate_session("first").await?, None);
    /// assert_eq!(service.validate_session("second").await?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revoke_all_sessions(&self, user_id: u32) -> Result<()> {
        self.repository
            .revoke_all_sessions(user_id)
            .await
            .map_err(map_common_error)
    }

    /// Deletes every expired session row across all users.
    ///
    /// Caller-dependent sweep (the library schedules no background work):
    /// a session past its expiry can never validate again, so its row is
    /// removed outright while every unexpired row — session AND refresh —
    /// survives. The refresh-domain counterpart is
    /// [`RefreshTokenService::cleanup_expired`]; each sweep stays inside
    /// its own token domain.
    ///
    /// # Errors
    ///
    /// * `RefreshTokenError::Common(CommonError::Database(_))` - database
    ///   failure.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::refresh_token::service::RefreshTokenService;
    /// use db_handler::db::connection::Connection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = Connection::new().await?;
    /// let service = RefreshTokenService::new(connection);
    /// service.cleanup_expired_sessions().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        self.repository
            .delete_expired_sessions(Utc::now())
            .await
            .map_err(map_common_error)
    }
}
#[cfg(test)]
mod tests;
