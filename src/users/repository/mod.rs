use crate::db::primary::{
    error::{CommonError, Result},
    models::{PaginatedResponse, PaginationParams},
    repository::{get_paginated_results_by_string, PrimaryDbRepository},
    service::map_sqlx_to_common_error,
};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::sync::Arc;

use crate::users::model::{User, UserSortKey};

use super::model::InternalUser;
use super::password::{password_length_ok, verify_password_async, DUMMY_ARGON2_HASH};

/// Repository for performing CRUD operations on `User` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so unique violations (duplicate email)
/// surface as the structured `CommonError::UniqueViolation` variant instead
/// of requiring callers to inspect error messages.
pub(crate) struct UserRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for UserRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl UserRepository {
    pub(crate) async fn insert(&self, user: &InternalUser) -> Result<u32> {
        let result = sqlx::query!(
            "INSERT INTO users (email, username,password_hash) VALUES (?, ?,?)",
            user.email,
            user.username,
            user.password_hash
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }

    pub(crate) async fn update_by_id(
        &self,
        id: u32,
        username: Option<String>,
        email: Option<String>,
        // Already Argon2id-hashed by the service: hashing is domain logic
        // and lives above the data-access layer, which only persists what
        // it is handed.
        password_hash: Option<String>,
    ) -> Result<u32> {
        let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE users SET ");
        let mut fields_updated = 0;

        if let Some(username) = username {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("username = ").push_bind(username);
            fields_updated += 1;
        }

        if let Some(email) = email {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("email = ").push_bind(email);
            fields_updated += 1;
        }

        if let Some(password_hash) = password_hash {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder
                .push("password_hash = ")
                .push_bind(password_hash);
            fields_updated += 1;
        }

        if fields_updated == 0 {
            return Err(CommonError::Operation(
                "At least one field (username, email, or password) must be provided for update"
                    .to_string(),
            ));
        }

        query_builder.push(" WHERE id = ").push_bind(id);

        let query = query_builder.build();
        let result = query
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;

        Ok(result.rows_affected() as u32)
    }

    pub(crate) async fn get_by_id(&self, id: u32) -> Result<Option<User>> {
        let result = sqlx::query!("SELECT id, username, email FROM users WHERE id = ?", id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;

        if let Some(record) = result {
            Ok(Some(User {
                id: record.id as u32,
                username: record.username,
                email: record.email,
            }))
        } else {
            Ok(None)
        }
    }

    /// Exact-email lookup (byte-exact, parameterized).
    ///
    /// One bound equality query (`WHERE email = ?`) with the same SQLite
    /// byte-exact semantics as the login path: no trimming, no lowercasing,
    /// no case folding, no pagination, no in-memory scan. Returns the
    /// public model, which structurally carries no password hash.
    ///
    /// Uses the runtime query API (like the tests' raw-row helpers): the
    /// compile-time macro would need a freshly generated `.sqlx` artifact,
    /// while the runtime API is offline-mode independent; every statement
    /// added here is exercised by the adjacent tests.
    pub(crate) async fn get_by_email(&self, email: String) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT id, username, email FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)
    }

    /// Atomic password reset invalidating every bearer credential of the
    /// user across BOTH token domains.
    ///
    /// ONE transaction, in order:
    /// 1. exact byte-comparable email lookup inside the transaction —
    ///    an unknown email writes nothing and reports `Ok(None)`;
    /// 2. `UPDATE users SET password_hash = ? WHERE id = ?` to the
    ///    freshly generated Argon2id PHC the service hands in;
    /// 3. `DELETE FROM refresh_tokens WHERE user_id = ?` — deliberately
    ///    UNSCOPED by domain (no `grpc-session:` predicate): a password
    ///    reset must kill both the REST refresh rows AND the gRPC session
    ///    rows of the user, unlike every other per-user sweep in this
    ///    crate which is domain-scoped on purpose;
    /// 4. COMMIT.
    ///
    /// Any lookup/update/deletion/commit error aborts the whole
    /// transaction (sqlx rolls the open transaction back on drop), leaving
    /// the old password and every refresh/session row intact. The
    /// memory-hard Argon2 derivation has already happened in the service,
    /// BEFORE this transaction opens, so no SQLite write lock is held
    /// during hashing.
    ///
    /// Returns the updated public user model (structurally hash-free), or
    /// `Ok(None)` when the exact email matches no user.
    ///
    /// Runtime query API for the same offline-artifact independence reason
    /// as `get_by_email` above; every statement is exercised by the
    /// adjacent tests.
    pub(crate) async fn reset_password_revoking_tokens(
        &self,
        email: &str,
        // Already Argon2id-hashed by the service: hashing is domain logic
        // and must stay outside the open transaction (no write lock is
        // held during memory-hard hashing). The repository only persists
        // the hash it is handed.
        password_hash: String,
    ) -> Result<Option<User>> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_to_common_error)?;

        // 1. Exact-email lookup inside the transaction (byte-comparable,
        //    bound parameter — same semantics as the login lookup).
        let record =
            sqlx::query_as::<_, User>("SELECT id, username, email FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_to_common_error)?;

        let Some(record) = record else {
            // Unknown email: nothing has been written; returning here
            // drops the open transaction and rolls it back.
            return Ok(None);
        };

        // 2. Password update inside the same transaction.
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(password_hash)
            .bind(record.id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_to_common_error)?;

        // 3. Cross-domain credential invalidation inside the same
        //    transaction: NO domain predicate here on purpose — both the
        //    refresh-domain and session-domain rows of this user are
        //    deleted, matching refresh replay-invalidation semantics.
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(record.id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_to_common_error)?;

        // 4. Commit: only now are the password change and the token
        //    purge visible together.
        tx.commit().await.map_err(map_sqlx_to_common_error)?;

        Ok(Some(record))
    }

    pub(crate) async fn get_by_email_and_password(
        &self,
        email: String,
        password: String,
    ) -> Result<Option<User>> {
        // Uniform length gate: an out-of-range password is rejected as
        // `None` BEFORE any SQL or Argon2 work, independently of whether
        // the byte-exact email exists, so an invalid length can never
        // reveal account existence.
        if !password_length_ok(&password) {
            return Ok(None);
        }

        let result = sqlx::query!(
            "SELECT id, username, email,password_hash FROM users WHERE email = ?",
            email
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;

        // Single always-verify path: Argon2 runs exactly once whether or
        // not the email exists. An unknown email verifies against a fixed
        // dummy PHC (same default policy, see `DUMMY_ARGON2_HASH`), so both
        // branches burn identical CPU and response time can no longer
        // reveal whether an email is registered (timing user-enumeration
        // oracle, CWE-208). The presented password is cloned first
        // (bounded to 8-128 bytes by the gate above) so a failed
        // verification can still burn the equalizing dummy derivation
        // below.
        let stored_hash = result.as_ref().map(|record| record.password_hash.clone());
        let verified = verify_password_async(
            password.clone(),
            stored_hash.unwrap_or_else(|| DUMMY_ARGON2_HASH.to_owned()),
        )
        .await;
        // Equalize the verification-ERROR path too (CWE-208): a stored
        // hash that fails PHC parsing — the shape of every pre-migration
        // or corrupted row — errors out before any Argon2 work, so its
        // response would be measurably faster than the wrong-password and
        // unknown-email responses and reveal that the email exists with
        // an unusable hash. Burn exactly one dummy verification (result
        // discarded) before reporting the failure uniformly: malformed
        // rows, unknown emails and wrong passwords all cost one
        // derivation.
        let verified = match verified {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                let _ = verify_password_async(password, DUMMY_ARGON2_HASH.to_owned()).await;
                Err(error)
            }
        };

        match (result, verified) {
            (Some(record), Ok(true)) => {
                let id = record.id.ok_or_else(|| {
                    CommonError::Operation("Missing id in database record".to_string())
                })? as u32;
                Ok(Some(User {
                    id,
                    username: record.username,
                    email: record.email,
                }))
            }
            // Any other combination — unknown email, wrong password, or a
            // malformed/failed verification — reports no user: fail closed
            // without panic and without exposing hash details.
            _ => Ok(None),
        }
    }

    pub(crate) async fn delete_by_id(&self, id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM users WHERE id = ?", id)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    pub(crate) async fn find_by_string(
        &self,
        pagination_params: &PaginationParams<UserSortKey>,
    ) -> Result<PaginatedResponse<User>> {
        // The shared pagination helper still reports errors as `anyhow`
        // (its rework is out of this module's scope); carry the message as
        // an operation error so the repository keeps a single error type.
        // `T` is pinned by the `PaginatedResponse<User>` return type;
        // the sort-key parameter `K` is inferred from `pagination_params`.
        get_paginated_results_by_string(&self.pool, "users", pagination_params)
            .await
            .map_err(|e| CommonError::Operation(e.to_string()))
    }
}

#[cfg(test)]
mod tests;
