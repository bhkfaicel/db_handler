use anyhow::Result;
use sqlx::{SqlitePool, Transaction};
use std::sync::Arc;

use crate::{db::primary::repository::PrimaryDbRepository, history::keys::model::HistoricalKey};
/// Repository for performing CRUD operations on `Historical Keys` entities.
pub(crate) struct HistoricalKeyRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for HistoricalKeyRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl HistoricalKeyRepository {
    pub(crate) async fn insert(&self, key: &HistoricalKey) -> Result<u32, sqlx::Error> {
        let value = key.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)",
            value,
            key.expire_at
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }

    /// Insert a historical key within a transaction.
    pub(crate) async fn insert_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        key: &HistoricalKey,
    ) -> Result<u32, sqlx::Error> {
        let value = key.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)",
            value,
            key.expire_at
        )
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }
}

#[cfg(test)]
mod tests;
