use crate::drives::columns::HasColumns;
use chrono::{DateTime, Utc};
use db_derive::HasColumns;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a refresh token stored in the database.
///
/// # Fields
///
/// - `id`: Auto-incremented unique identifier.
/// - `user_id`: User identifier.
/// - `token`: Refresh token value.
/// - `created_at`: UTC timestamp at which the token was created.
/// - `expire_at`: UTC timestamp at which the token becomes invalid.
/// - `used`: Used flag.
#[derive(Debug, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub struct RefreshToken {
    /// Unique refresh token identifier.
    pub id: u32,
    /// User identifier.
    pub user_id: u32,
    /// Refresh token value.
    pub token: String,
    /// UTC timestamp at which the token becomes invalid.
    pub expire_at: DateTime<Utc>,
    /// UTC timestamp at which the token was created.
    pub created_at: DateTime<Utc>,
    /// Used flag.
    pub is_used: bool,
}

// Behavioral tests for the refresh token data model live in their own
// directory next to the source file, per the project test layout rule.
#[cfg(test)]
mod tests;
