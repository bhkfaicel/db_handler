use crate::drives::columns::HasColumns;
use chrono::{DateTime, Utc};
use db_derive::HasColumns;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

/// Represents a key stored in the database.
///
/// # Fields
///
/// - `id`: Auto-incremented unique identifier.
/// - `value`: Raw key data (BLOB) of fixed length (32 bytes).
/// - `expire_at`: UTC timestamp at which the key becomes invalid.
/// - `mac`: Mac of sensor.
#[derive(FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Unique key identifier.
    pub id: u32,
    /// Key value as a byte vector (32 bytes). Secret key material:
    /// serialization skips this field so the raw bytes never travel
    /// through a serialized form of the model, while deserialization
    /// keeps accepting a complete payload.
    #[serde(skip_serializing)]
    pub value: Vec<u8>,
    /// UTC timestamp at which the key becomes invalid.
    pub expire_at: DateTime<Utc>,
    /// Unique mac. The format (six hexadecimal pairs separated by `:`,
    /// case-insensitive) is enforced at deserialization: the fields are
    /// public, so constructor-side checks alone would be bypassable by
    /// deserializing an untrusted payload.
    #[serde(deserialize_with = "crate::common::helpers::mac::deserialize")]
    pub mac: String,
}

// Hand-written Debug: the key bytes are secret key material, so they
// render as a fixed marker instead of their value and no log line, panic
// message or debug dump can ever leak them. Every other field keeps its
// normal readable rendering.
impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Key")
            .field("id", &self.id)
            .field("value", &"[REDACTED]")
            .field("expire_at", &self.expire_at)
            .field("mac", &self.mac)
            .finish()
    }
}

// Behavioral tests for the key data model live in their own directory
// next to the source file, per the project test layout rule.
#[cfg(test)]
mod tests;
