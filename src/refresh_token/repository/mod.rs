use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::db::primary::error::Result;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::refresh_token::error::RotationOutcome;
use crate::refresh_token::model::RefreshToken;

/// Computes the SHA-256 hex digest of a caller-supplied token value.
///
/// Single at-rest hashing point: every store and lookup path converts the
/// plaintext token to its 64-character lowercase hex digest here, BEFORE it
/// reaches SQL, so the database never stores a usable bearer token (a DB
/// read alone cannot yield a usable session). Crate-private on purpose: no
/// public getter ever returns a stored digest, and callers keep handing
/// over the plaintext token string.
pub(crate) fn token_digest(token: &str) -> String {
    // 32 raw bytes expanded to 64 lowercase hex characters.
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Repository for performing CRUD operations on `RefreshToken` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so unique violations surface as the
/// structured `CommonError::UniqueViolation` variant instead of requiring
/// callers to inspect error messages.
///
/// At rest, tokens exist only as SHA-256 digests (`token_digest`): the
/// plaintext value supplied by callers never reaches the database.
pub(crate) struct RefreshTokenRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for RefreshTokenRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

impl RefreshTokenRepository {
    /// Insert a refresh token.
    ///
    /// The token is stored under its SHA-256 hex digest: the plaintext
    /// value never reaches the database. A duplicate caller-supplied token
    /// collides on its digest and surfaces as
    /// `CommonError::UniqueViolation` (the `token` column stays UNIQUE);
    /// foreign-key failures (unknown user) keep the underlying sqlx error
    /// inside the generic `Database` variant.
    pub(crate) async fn insert(&self, refresh_token: &RefreshToken) -> Result<u32> {
        // The digest is bound to a local: the query borrows its arguments.
        let digest = token_digest(&refresh_token.token);
        let result = sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token, expire_at, created_at, is_used) VALUES (?, ?, ?, ?, ?)",
            refresh_token.user_id,
            digest,
            refresh_token.expire_at,
            refresh_token.created_at,
           false
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }
    pub(crate) async fn delete_by_user_id(&self, user_id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = ?", user_id)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
    /// Atomically consume `old_token` and insert `new_token` as its
    /// replacement.
    ///
    /// Both tokens are hashed (SHA-256) before they reach SQL: the
    /// consumption UPDATE and the classification SELECT match on the old
    /// token's digest, and the replacement is stored under its digest. The
    /// returned `Rotated` model echoes the caller-supplied replacement
    /// value (input echo), never a stored digest.
    ///
    /// ONE transaction drives every step (SQLite serializes writers, so two
    /// rotation transactions never interleave):
    ///
    /// 1. conditional consumption: `UPDATE ... SET is_used = true WHERE
    ///    user_id = ? AND token = ? AND is_used = false AND expire_at > ?`;
    /// 2. one row affected: the replacement is inserted in the same
    ///    transaction and `Rotated` is returned. A unique violation on the
    ///    replacement's token aborts the transaction, rolling the
    ///    consumption back and leaving the old token usable;
    /// 3. zero rows: a follow-up lookup in the same transaction classifies
    ///    the outcome — no row means `NotFound`, an expired row means
    ///    `Expired`, an already-used row means a replay: every token of the
    ///    user is deleted (reuse mitigation) and `ReuseDetected` is
    ///    returned.
    ///
    /// Reuse-detected atomicity: SQLite allows only one writer at a time, so
    /// a reuse purge either commits before the winner's consumption (the
    /// winner's UPDATE then matches zero rows and inserts no replacement) or
    /// after it (the purge then deletes the replacement along with the rest
    /// of the user's tokens). A half-consumed, half-replaced state is
    /// impossible.
    ///
    /// The expired-only per-user purge runs inside this transaction, at its
    /// end and after the classification: the just-consumed token has
    /// `expire_at > now` (that is why the conditional UPDATE matched), so it
    /// survives the purge and stays available for replay detection until its
    /// natural expiry.
    pub(crate) async fn rotate(
        &self,
        user_id: u32,
        old_token: &str,
        new_token: &RefreshToken,
        now: DateTime<Utc>,
    ) -> Result<RotationOutcome> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_to_common_error)?;
        // Hash once at entry, then use the digests throughout the
        // transaction (the old token is consumed and classified by digest,
        // the replacement is stored by digest).
        let old_digest = token_digest(old_token);
        // Conditional consumption: the row is flagged used only when it
        // belongs to the user, matches the token, was never consumed and has
        // not expired.
        let consumed = sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE user_id = ? AND token = ? AND is_used = false AND expire_at > ?",
            user_id,
            old_digest,
            now
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_to_common_error)?
        .rows_affected();

        let outcome = if consumed == 1 {
            // Replacement insert in the SAME transaction, under the new
            // token's digest (bound to a local: the query borrows its
            // arguments).
            let new_digest = token_digest(&new_token.token);
            let insert = sqlx::query!(
                "INSERT INTO refresh_tokens (user_id, token, expire_at, created_at, is_used) VALUES (?, ?, ?, ?, ?)",
                new_token.user_id,
                new_digest,
                new_token.expire_at,
                new_token.created_at,
                false
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_to_common_error)?;
            // Input echo: the model carries the caller-supplied value; only
            // the digest was stored.
            RotationOutcome::Rotated(RefreshToken {
                id: insert.last_insert_rowid() as u32,
                user_id: new_token.user_id,
                token: new_token.token.clone(),
                expire_at: new_token.expire_at,
                created_at: new_token.created_at,
                is_used: false,
            })
        } else {
            // Zero-row classification, in the same transaction: the row
            // state the UPDATE saw is exactly what this SELECT sees (a
            // single writer holds the write lock until COMMIT).
            let record = sqlx::query!(
                "SELECT is_used, expire_at FROM refresh_tokens WHERE user_id = ? AND token = ?",
                user_id,
                old_digest
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_to_common_error)?;
            match record {
                None => RotationOutcome::NotFound,
                Some(record) if record.expire_at <= now.naive_utc() => RotationOutcome::Expired,
                Some(record) => {
                    // An unused, unexpired row always matches the
                    // conditional UPDATE above, and no other writer can
                    // commit between that UPDATE and this SELECT (SQLite
                    // holds the single write lock until COMMIT), so an
                    // unused row reaching this arm is impossible.
                    assert!(
                        record.is_used,
                        "rotation classified an unused, unexpired token that the conditional consumption did not match"
                    );
                    // Replay detected: delete every token of the user.
                    sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = ?", user_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(map_sqlx_to_common_error)?;
                    RotationOutcome::ReuseDetected
                }
            }
        };

        // Expired-only retention purge, inside the transaction, at its end
        // and after the classification (and after the replacement insert on
        // the rotated path): already-used tokens are retained until their
        // natural expiry so replays keep being detected.
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = ? AND expire_at < ?",
            user_id,
            now
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_to_common_error)?;

        tx.commit().await.map_err(map_sqlx_to_common_error)?;
        Ok(outcome)
    }

    /// Delete the EXPIRED refresh tokens of one user (expired-only policy).
    ///
    /// Already-used tokens are deliberately retained until their natural
    /// expiry: they must stay queryable so a replayed consumed token keeps
    /// classifying as reuse (`RotationOutcome::ReuseDetected`) for the
    /// token's full lifetime. Per-user growth stays bounded by the expiry
    /// window, not by the number of rotations. Executed on the token-issuance
    /// path (`RefreshTokenService::add`) before the insert and inside the
    /// rotation transaction after the old-token classification. The
    /// `user_id` predicate rides the existing `idx_refresh_tokens_user_id`
    /// index.
    pub(crate) async fn delete_expired_for_user(
        &self,
        user_id: u32,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = ? AND expire_at < ?",
            user_id,
            now
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Delete every expired refresh token across all users.
    ///
    /// Global sweep behind the public `RefreshTokenService::cleanup_expired`
    /// hook: token issuance only purges the logging-in user's rows, so
    /// tokens of users who never log in again are reached exclusively by
    /// this explicit, caller-dependent sweep.
    pub(crate) async fn delete_expired(&self, now: DateTime<Utc>) -> Result<()> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE expire_at < ?", now)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
