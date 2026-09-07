//! Anti-replay nonce ledger.
//!
//! Ledger contract: a nonce value is burned GLOBALLY once accepted — the
//! same value is refused afterwards no matter which sensor presents it.
//! Detection is enforced by the database itself: the column-level
//! `UNIQUE(value)` constraint rejects the duplicate inside the INSERT, so
//! concurrent writers observe exactly one acceptance and no
//! SELECT-then-INSERT probe exists that a race could defeat. Accepted rows
//! are never deleted on verification; they are retained until the
//! presenting sensor's key row is deleted at key rotation, whose
//! `ON DELETE CASCADE` foreign key removes that sensor's nonces, so ledger
//! growth stays bounded by the key-rotation window of each sensor. With
//! 96-bit values, two distinct sensors independently generating the same
//! nonce has probability about n²/2⁹⁷ (roughly 6e-18 at one million nonces
//! per window), so the global scope never rejects legitimate traffic in
//! practice. There is deliberately no explicit purge method: the cascade
//! at key rotation is the only deletion path.

pub mod error;
pub mod model;
mod repository;
pub mod service;
