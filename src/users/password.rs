//! Password hashing and verification helpers for the `users` domain.
//!
//! User-authentication logic lives in the domain that owns it, not in the
//! core `common` layer. Both the service (hashing before persistence) and
//! the repository (credential verification) call these helpers, so they sit
//! at the module root instead of inside one of them.

use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};

/// Hashes a clear-text password with bcrypt (default cost).
///
/// Synchronous by design: callers keep the current sync-call semantics.
pub(crate) fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

/// Verifies a clear-text password against a stored bcrypt hash.
pub(crate) fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let is_valid = verify(password, hash)?;
    Ok(is_valid)
}

#[cfg(test)]
mod tests;
