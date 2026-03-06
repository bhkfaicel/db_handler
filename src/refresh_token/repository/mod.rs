use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::{db::primary::repository::PrimaryDbRepository, refresh_token::model::RefreshToken};
/// Repository for performing CRUD operations on `RefreshToken` entities.
pub(crate) struct RefreshTokenRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for RefreshTokenRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

impl RefreshTokenRepository {
    pub(crate) async fn insert(&self, refresh_token: &RefreshToken) -> Result<u32, sqlx::Error> {
        let result = sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token, expire_at, created_at, is_used) VALUES (?, ?, ?, ?, ?)",
            refresh_token.user_id,
            refresh_token.token,
            refresh_token.expire_at,
            refresh_token.created_at,
           false
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }
    pub(crate) async fn get_by_user_id_and_token(
        &self,
        user_id: u32,
        token: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT * FROM refresh_tokens WHERE user_id = ? AND token = ?",
            user_id,
            token
        )
        .fetch_optional(&*self.pool)
        .await?;
        if let Some(record) = result {
            let id = record.id.ok_or_else(|| sqlx::Error::RowNotFound)? as u32;
            Ok(Some(RefreshToken {
                id: id,
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
    pub(crate) async fn delete_by_user_id(&self, user_id: u32) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = ?", user_id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub(crate) async fn update_flag_by_token(&self, token: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE refresh_tokens SET is_used = true WHERE token = ?",
            token
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
