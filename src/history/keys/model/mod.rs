use crate::drives::columns::HasColumns;
use chrono::{DateTime, Utc};
use db_derive::HasColumns;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents an historical key stored in the database.
///
/// # Fields
///
/// - `id`: Auto-incremented unique identifier.
/// - `value`: Raw key data (BLOB) of fixed length (32 bytes).
/// - `expire_at`: UTC timestamp at which the key becomes invalid.
#[derive(Debug, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub struct HistoricalKey {
    /// Unique key identifier.
    pub id: u32,
    /// Key value as a byte vector (32 bytes).
    pub value: Vec<u8>,
    /// UTC timestamp at which the key becomes invalid.
    pub expire_at: DateTime<Utc>,
}
