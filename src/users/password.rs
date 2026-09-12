//! Argon2id password hashing and verification helpers for the `users`
//! domain.
//!
//! User-authentication logic lives in the domain that owns it, not in the
//! core `common` layer. Both the service (hashing before persistence) and
//! the repository (credential verification) call these helpers, so they sit
//! at the module root instead of inside one of them.
//!
//! Hashes are stored as standard PHC strings produced by RustCrypto's
//! Argon2id with the crate default parameters (`v=19`, `m=19456`, `t=2`,
//! `p=1`) and a unique OS-CSPRNG salt per hash. Verification parses the
//! stored PHC strictly and fails closed on any malformed input. Each helper
//! has two shapes: the synchronous primitive is the bare Argon2 call (kept
//! for non-async reuse and unit tests); the `*_async` wrappers run it on the
//! tokio blocking pool, because a memory-hard Argon2id derivation burns
//! tens of milliseconds of CPU and memory that must never stall an async
//! worker.

use anyhow::Result;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use password_hash::{rand_core::OsRng, SaltString};

/// Minimum accepted clear-password length in bytes.
pub(crate) const MIN_PASSWORD_BYTES: usize = 8;
/// Maximum accepted clear-password length in bytes. Caps the input fronted
/// to the intentionally expensive memory-hard derivation so an exposed
/// caller cannot amplify CPU/memory pressure with oversized secrets.
pub(crate) const MAX_PASSWORD_BYTES: usize = 128;

/// Fixed dummy PHC used by the login lookup when the email does not exist.
///
/// A valid Argon2id PHC generated with the SAME default policy as real
/// stored hashes (`Argon2::default`): verifying a presented password
/// against it lets the unknown-email login branch burn exactly the same
/// CPU/memory as the known-email branch, so response timing cannot
/// distinguish a registered email from an unknown one (timing
/// user-enumeration oracle, CWE-208). The clear-text source string was
/// deliberately discarded after generation; no realistic password verifies
/// against this constant.
pub(crate) const DUMMY_ARGON2_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$LVqSDkuc8AaNn2RY/XKdHA$RexxtUuD7oN3tD2xhnxvYTM9oL/+RFTRzrqYv7GD5PE";

/// Ensures a clear password is inside the accepted length bounds.
///
/// Returns `true` when the password length in bytes is
/// `MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES`; callers use this to reject
/// oversized/undersized secrets before any Argon2 work.
pub(crate) fn password_length_ok(password: &str) -> bool {
    // `str::len()` is the length in bytes, exactly what the policy needs.
    let length = password.len();
    (MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES).contains(&length)
}

/// Hashes a clear-text password with Argon2id and a unique random salt.
///
/// Synchronous primitive: async paths must use `hash_password_async`.
/// Out-of-bounds passwords are rejected here as well so no caller can
/// bypass the policy by using the primitive directly.
pub(crate) fn hash_password(password: &str) -> Result<String> {
    if !password_length_ok(password) {
        anyhow::bail!(
            "password length must be between {} and {} bytes",
            MIN_PASSWORD_BYTES,
            MAX_PASSWORD_BYTES
        );
    }
    // Unique OS-CSPRNG salt per hash: two hashes of one password are never
    // identical, preventing precomputed-hash comparison and rainbow reuse.
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        // `password_hash::Error` is not `std::error::Error`, so carry only
        // its failure message (it never contains the hash or password).
        .map_err(|e| anyhow::anyhow!("password hash failed: {e}"))
}

/// Verifies a clear-text password against a stored Argon2id PHC string.
///
/// Synchronous primitive: async paths must use `verify_password_async`.
/// Strict PHC parsing: a malformed or unparsable stored hash is an error,
/// so callers can fail closed instead of silently accepting a user.
pub(crate) fn verify_password(password: &str, hash: &str) -> Result<bool> {
    // Strict PHC parsing: an unparsable stored string is a hard error so
    // callers fail closed; only the parse-failure reason is carried (the
    // stored hash itself is never embedded in the message).
    let parsed =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Hashes a clear-text password with Argon2id off the async runtime.
///
/// The owned `String` is moved into `tokio::task::spawn_blocking` so the
/// memory-hard derivation runs on the dedicated blocking pool instead of
/// stalling the calling tokio worker; the hash string is moved back out and
/// the Argon2 result is returned unchanged.
pub(crate) async fn hash_password_async(password: String) -> Result<String> {
    match tokio::task::spawn_blocking(move || hash_password(&password)).await {
        // The inner result is the plain Argon2 outcome.
        Ok(result) => result,
        // The outer result can only fail if the blocking task itself died
        // (panic); surface that as a regular hashing failure.
        Err(join_error) => Err(anyhow::Error::new(join_error)),
    }
}

/// Verifies a clear-text password against a stored Argon2id PHC string off
/// the async runtime.
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
