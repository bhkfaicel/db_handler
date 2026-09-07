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

/// Domain tag carried by every stored gRPC session digest.
///
/// The refresh domain stores bare 64-hex digests; the session domain
/// stores `<tag><64-hex>`. The tag contains `:` and letters outside the
/// hexadecimal alphabet, so a refresh digest can never equal a session
/// value and vice versa: the stored format alone separates the two token
/// domains, and every SQL path below enforces it with the matching
/// `LIKE`/`NOT LIKE 'grpc-session:%'` predicate. Keep this constant and
/// the LIKE literals inside the queries in sync.
pub(crate) const SESSION_DIGEST_PREFIX: &str = "grpc-session:";

/// Computes the domain-tagged digest of a caller-supplied session token.
///
/// Session counterpart of [`token_digest`]: the `grpc-session:` prefix
/// makes a session row unmatchable by every refresh-domain path and a
/// refresh row unmatchable by every session path, with no schema change
/// (the `token` column stays `TEXT NOT NULL UNIQUE`, so duplicates inside
/// one domain still surface as a unique violation while the same raw value
/// stored in both domains yields two distinct rows). The plaintext token
/// never reaches SQL: only this tagged digest is stored or matched.
pub(crate) fn session_digest(token: &str) -> String {
    format!("{SESSION_DIGEST_PREFIX}{}", token_digest(token))
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
    ///
    /// Domain scoping is by construction: the bare 64-hex digest can never
    /// carry the `grpc-session:` tag, so this insert can never create a
    /// session-domain row.
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
        // Refresh-domain-only predicate: session rows survive this sweep
        // (the `user_id` filter rides the per-user index, the LIKE is a
        // residual filter on the matched rows).
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = ? AND token NOT LIKE 'grpc-session:%'",
            user_id
        )
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
        // belongs to the user, matches the token, was never consumed and
        // has not expired. The digest bound to `token = ?` is a bare 64-hex
        // value that can never equal a `grpc-session:`-tagged session row;
        // the NOT LIKE predicate is the explicit refresh-domain guard the
        // rest of the refresh paths carry (residual filter behind the
        // unique-index equality lookup).
        let consumed = sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE user_id = ? AND token = ? AND is_used = false AND expire_at > ? AND token NOT LIKE 'grpc-session:%'",
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
            // single writer holds the write lock until COMMIT). The
            // refresh-domain predicate mirrors the consumption UPDATE.
            let record = sqlx::query!(
                "SELECT is_used, expire_at FROM refresh_tokens WHERE user_id = ? AND token = ? AND token NOT LIKE 'grpc-session:%'",
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
                    // Replay detected: delete every REFRESH token of the
                    // user (session-domain rows are out of this purge's
                    // reach: revoking gRPC sessions is never a side effect
                    // of REST replay mitigation).
                    sqlx::query!(
                        "DELETE FROM refresh_tokens WHERE user_id = ? AND token NOT LIKE 'grpc-session:%'",
                        user_id
                    )
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
        // natural expiry so replays keep being detected. Refresh-domain
        // predicate: the purge never touches session rows.
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = ? AND expire_at < ? AND token NOT LIKE 'grpc-session:%'",
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
    /// index. Refresh-domain predicate: session rows of the user are never
    /// deleted by this issuance-path sweep.
    pub(crate) async fn delete_expired_for_user(
        &self,
        user_id: u32,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = ? AND expire_at < ? AND token NOT LIKE 'grpc-session:%'",
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
    /// this explicit, caller-dependent sweep. Refresh-domain predicate:
    /// expired session rows are left to `delete_expired_sessions`.
    pub(crate) async fn delete_expired(&self, now: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE expire_at < ? AND token NOT LIKE 'grpc-session:%'",
            now
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Insert a gRPC session row for `(user_id, raw_token)`.
    ///
    /// The session is stored under its domain-tagged digest
    /// (`grpc-session:<sha256(raw)>`), so the plaintext never reaches SQL
    /// and the row is unmatchable by every refresh-domain path. The
    /// `is_used` flag is born FALSE; revocation flips it in one atomic
    /// UPDATE (`revoke_session`). A duplicate raw token (same tagged digest
    /// already stored) violates the column UNIQUE and surfaces as
    /// `CommonError::UniqueViolation`, which the service maps to the typed
    /// `RefreshTokenError::AlreadyExists` — no overwrite, no expiry
    /// extension.
    pub(crate) async fn insert_session(
        &self,
        user_id: u32,
        token: &str,
        expire_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        // Digest bound to a local: the query borrows its arguments.
        let digest = session_digest(token);
        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token, expire_at, created_at, is_used) VALUES (?, ?, ?, ?, FALSE)",
            user_id,
            digest,
            expire_at,
            created_at
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Non-consuming validation of a presented session token.
    ///
    /// Returns the owning user ID when ONE session-domain row matches the
    /// tagged digest of the presented value AND is unrevoked
    /// (`is_used = false`) AND strictly unexpired (`expire_at > now`): the
    /// whole decision lives in the SQL predicate. The row is never mutated,
    /// so validation repeats until revocation or expiry. The `token = ?`
    /// equality rides the UNIQUE column constraint's implicit index (the
    /// tagged digest can never equal a bare refresh digest by construction)
    /// and the `LIKE 'grpc-session:%'` predicate is the explicit
    /// session-domain guard every session path carries.
    pub(crate) async fn session_user_id(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<u32>> {
        // Digest bound to a local: the query borrows its arguments.
        let digest = session_digest(token);
        let record = sqlx::query!(
            "SELECT user_id FROM refresh_tokens WHERE token = ? AND is_used = false AND expire_at > ? AND token LIKE 'grpc-session:%'",
            digest,
            now
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(record.map(|record| record.user_id as u32))
    }

    /// Revoke the session matching the presented raw token.
    ///
    /// ONE atomic statement: `is_used` flips to true for the session-domain
    /// row carrying the tagged digest. Idempotent by design — an unknown,
    /// already-revoked or expired presented value matches zero rows and
    /// still reports success, so logout never errors on a stale token. The
    /// session-domain predicate makes every refresh row unmatchable.
    pub(crate) async fn revoke_session(&self, token: &str) -> Result<()> {
        // Digest bound to a local: the query borrows its arguments.
        let digest = session_digest(token);
        sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE token = ? AND token LIKE 'grpc-session:%'",
            digest
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Revoke every session of one user in ONE atomic statement.
    ///
    /// Logout-everywhere: only session-domain rows of `user_id` flip to
    /// `is_used = true`; refresh rows (consumed or live) are untouched, so
    /// REST refresh rotation keeps working across a gRPC logout-all. The
    /// `user_id` predicate rides the `idx_refresh_tokens_user_id` index
    /// and the domain predicate filters the matched rows.
    pub(crate) async fn revoke_all_sessions(&self, user_id: u32) -> Result<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE user_id = ? AND token LIKE 'grpc-session:%'",
            user_id
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Delete the EXPIRED session rows of every user.
    ///
    /// Session counterpart of `delete_expired`: after expiry a session row
    /// can never validate again (revoked or not), so expired session rows
    /// are removed outright while every unexpired row — session AND refresh
    /// — survives. Caller-dependent sweep behind the public
    /// `RefreshTokenService::cleanup_expired_sessions` hook; the
    /// session-domain predicate keeps refresh rows out of its reach.
    pub(crate) async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE expire_at < ? AND token LIKE 'grpc-session:%'",
            now
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
