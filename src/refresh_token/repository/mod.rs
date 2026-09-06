use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::db::primary::error::Result;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::refresh_token::model::RefreshToken;
/// Repository for performing CRUD operations on `RefreshToken` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so unique violations surface as the
/// structured `CommonError::UniqueViolation` variant instead of requiring
/// callers to inspect error messages.
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
    /// A duplicate token (the `token` column is declared UNIQUE in the
    /// schema) surfaces as `CommonError::UniqueViolation`; foreign-key
    /// failures (unknown user) keep the underlying sqlx error inside the
    /// generic `Database` variant.
    pub(crate) async fn insert(&self, refresh_token: &RefreshToken) -> Result<u32> {
        let result = sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token, expire_at, created_at, is_used) VALUES (?, ?, ?, ?, ?)",
            refresh_token.user_id,
            refresh_token.token,
            refresh_token.expire_at,
            refresh_token.created_at,
           false
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }
    pub(crate) async fn get_by_user_id_and_token(
        &self,
        user_id: u32,
        token: &str,
    ) -> Result<Option<RefreshToken>> {
        let result = sqlx::query!(
            "SELECT * FROM refresh_tokens WHERE user_id = ? AND token = ?",
            user_id,
            token
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        if let Some(record) = result {
            let id = record
                .id
                .ok_or_else(|| map_sqlx_to_common_error(sqlx::Error::RowNotFound))?
                as u32;
            Ok(Some(RefreshToken {
                id,
                user_id: record.user_id as u32,
                token: record.token,
                expire_at: DateTime::<Utc>::from_naive_utc_and_offset(record.expire_at, Utc),
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(record.created_at, Utc),
                is_used: record.is_used,
            }))
        } else {
            Ok(None)
        }
    }
    pub(crate) async fn delete_by_user_id(&self, user_id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = ?", user_id)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
    pub(crate) async fn update_flag_by_token(&self, token: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE token = ?",
            token
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
