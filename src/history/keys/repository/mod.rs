use crate::db::primary::error::Result;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::history::keys::model::HistoricalKey;
use chrono::{DateTime, Utc};
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

    /// Delete every historical key whose retention window has elapsed,
    /// within the caller's transaction.
    ///
    /// Executed on the key-rotation path (`KeyService::add`), which is the
    /// only writer of this table, so the steady state is bounded by the
    /// 30-day retention window (one rotation per hour: about 720 rows per
    /// sensor). The DELETE is a full scan of that bounded table
    /// (sub-millisecond at this size); no index on `expire_at` is added on
    /// purpose: it would cost one extra B-tree write per insert to
    /// accelerate a scan that stays negligible. Revisit only with a
    /// measured regression.
    ///
    /// The `NOT EXISTS` guard is the foreign-key safety belt:
    /// `keys.value` references `historical_keys.value` with NO ACTION, so
    /// deleting a historical row still referenced by a key row would abort
    /// the whole purge with a foreign-key violation. The guard skips any
    /// expired row still referenced by a LIVE key; such a row becomes
    /// unreferenced — and therefore purgeable — once that key expires and
    /// the expired-keys purge (which runs first in the rotation
    /// transaction) removes it. With this guard no statement order can
    /// orphan a referenced row.
    pub(crate) async fn delete_expired_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM historical_keys WHERE expire_at < ? \
             AND NOT EXISTS (SELECT 1 FROM keys WHERE keys.value = historical_keys.value)",
            now
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
