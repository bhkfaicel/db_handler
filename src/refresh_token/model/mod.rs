use crate::drives::columns::HasColumns;
use chrono::{DateTime, Utc};
use db_derive::HasColumns;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

/// Represents a refresh token stored in the database.
///
/// # At-rest semantics split (`token` field)
///
/// `token` carries the CALLER-SUPPLIED value: `add` and `rotate` echo it
/// back in the returned model. It is NOT what the database stores: the
/// repository persists only the SHA-256 hex digest of the token, and no
/// public getter returns stored digests. Presenting the original
/// caller-supplied value to `rotate` remains the only way to consume a
/// token.
///
/// # Fields
///
/// - `id`: Auto-incremented unique identifier.
/// - `user_id`: User identifier.
/// - `token`: Caller-supplied refresh token value (input echo; the stored
///   column holds its SHA-256 digest).
/// - `created_at`: UTC timestamp at which the token was created.
/// - `expire_at`: UTC timestamp at which the token becomes invalid.
/// - `used`: Used flag.
#[derive(FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub struct RefreshToken {
    /// Unique refresh token identifier.
    pub id: u32,
    /// User identifier.
    pub user_id: u32,
    /// Caller-supplied refresh token value (input echo; the database
    /// stores only its SHA-256 digest). A credential: serialization skips
    /// this field so the secret never travels through a serialized form
    /// of the model, while deserialization keeps accepting a complete
    /// payload.
    #[serde(skip_serializing)]
    pub token: String,
    /// UTC timestamp at which the token becomes invalid.
    pub expire_at: DateTime<Utc>,
    /// UTC timestamp at which the token was created.
    pub created_at: DateTime<Utc>,
    /// Used flag.
    pub is_used: bool,
}

// Hand-written Debug: the token value is a credential, so it renders as a
// fixed marker instead of its value and no log line, panic message or
// debug dump can ever leak it. Every other field keeps its normal
// readable rendering.
impl fmt::Debug for RefreshToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshToken")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("token", &"[REDACTED]")
            .field("expire_at", &self.expire_at)
            .field("created_at", &self.created_at)
            .field("is_used", &self.is_used)
            .finish()
    }
}

// Behavioral tests for the refresh token data model live in their own
// directory next to the source file, per the project test layout rule.
#[cfg(test)]
mod tests;
