use crate::drives::columns::HasColumns;
use db_derive::HasColumns;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a nonce stored in the database.
///
/// # Fields
///
/// - `id`: Auto-incremented unique identifier.
/// - `value`: Raw nonce data (BLOB) of fixed length (12 bytes).
/// - `mac`: sensor's mac.
#[derive(Debug, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub struct Nonce {
    /// Unique nonce identifier.
    pub id: u32,
    /// Nonce value as a byte vector (12 bytes).
    pub value: Vec<u8>,
    /// sensor mac.
    pub mac: String,
}
