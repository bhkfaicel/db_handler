//! Password hashing and verification helpers for the `users` domain.
//!
//! User-authentication logic lives in the domain that owns it, not in the
//! core `common` layer. Both the service (hashing before persistence) and
//! the repository (credential verification) call these helpers, so they sit
//! at the module root instead of inside one of them.
//!
//! Each helper has two shapes: the synchronous primitive is the bare bcrypt
//! call (kept for non-async reuse and unit tests); the `*_async` wrappers
//! run it on the tokio blocking pool, because a cost-12 bcrypt call burns
//! roughly 100-300 ms of pure CPU that must never stall an async worker.

use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};

/// Fixed dummy hash used by the login lookup when the email does not exist.
///
/// A valid bcrypt hash of a random 32-character string, generated at the
/// SAME cost policy as real stored hashes (`DEFAULT_COST`): verifying a
/// presented password against it lets the unknown-email login branch burn
/// exactly the same CPU as the known-email branch, so response timing
/// cannot distinguish a registered email from an unknown one (timing
/// user-enumeration oracle, CWE-208). The clear-text source string is
/// deliberately discarded after generation; no realistic password verifies
/// against this constant.
pub(crate) const DUMMY_BCRYPT_HASH: &str =
    "$2b$12$ejBn2v/cqgDnpTh.TI9PJeF7It.dktAeBplnlIrwnj8EM42Ln7km6";

/// Hashes a clear-text password with bcrypt (default cost).
///
/// Synchronous primitive: async paths must use `hash_password_async`.
pub(crate) fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

/// Verifies a clear-text password against a stored bcrypt hash.
///
/// Synchronous primitive: async paths must use `verify_password_async`.
pub(crate) fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let verified = verify(password, hash)?;
    Ok(verified)
}

/// Hashes a clear-text password with bcrypt off the async runtime.
///
/// The owned `String` is moved into `tokio::task::spawn_blocking` so the
/// ~100-300 ms of CPU a cost-12 hash costs runs on the dedicated blocking
/// pool instead of stalling the calling tokio worker; the hash string is
/// moved back out and the bcrypt result is returned unchanged.
pub(crate) async fn hash_password_async(password: String) -> Result<String> {
    match tokio::task::spawn_blocking(move || hash_password(&password)).await {
        // The inner result is the plain bcrypt outcome.
        Ok(result) => result,
        // The outer result can only fail if the blocking task itself died
        // (panic); surface that as a regular hashing failure.
        Err(join_error) => Err(anyhow::Error::new(join_error)),
    }
}

/// Verifies a clear-text password against a stored bcrypt hash off the async
/// runtime.
///
/// Both owned inputs are moved into `tokio::task::spawn_blocking` for the
/// same runtime-starvation reason as `hash_password_async`; the boolean
/// verdict is returned unchanged.
pub(crate) async fn verify_password_async(password: String, hash: String) -> Result<bool> {
    match tokio::task::spawn_blocking(move || verify_password(&password, &hash)).await {
        Ok(result) => result,
        Err(join_error) => Err(anyhow::Error::new(join_error)),
    }
}

#[cfg(test)]
mod tests;
