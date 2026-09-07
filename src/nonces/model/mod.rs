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
    /// Sensor mac. The format (six hexadecimal pairs separated by `:`,
    /// case-insensitive) is enforced at deserialization: the fields are
    /// public, so constructor-side checks alone would be bypassable by
    /// deserializing an untrusted payload.
    #[serde(deserialize_with = "crate::common::helpers::mac::deserialize")]
    pub mac: String,
}

// Behavioral tests for the nonce data model live in their own directory
// next to the source file, per the project test layout rule.
#[cfg(test)]
mod tests;
