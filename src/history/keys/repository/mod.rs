use crate::db::primary::error::Result;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::history::keys::model::HistoricalKey;
use sqlx::{SqlitePool, Transaction};
use std::sync::Arc;

/// Repository for performing CRUD operations on `Historical Keys` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so a duplicate `value` (the column is
/// UNIQUE in the schema) surfaces as the structured
/// `CommonError::UniqueViolation` variant instead of a raw sqlx error.
pub(crate) struct HistoricalKeyRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for HistoricalKeyRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl HistoricalKeyRepository {
    pub(crate) async fn insert(&self, key: &HistoricalKey) -> Result<u32> {
        let value = key.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)",
            value,
            key.expire_at
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }

    /// Insert a historical key within a transaction.
    pub(crate) async fn insert_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        key: &HistoricalKey,
    ) -> Result<u32> {
        let value = key.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)",
            value,
            key.expire_at
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }
}

#[cfg(test)]
mod tests;
