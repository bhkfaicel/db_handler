use crate::{db::primary::repository::PrimaryDbRepository, keys::model::Key};
use chrono::{DateTime, Utc};
use sqlx::{SqlitePool, Transaction};
use std::sync::Arc;
/// Repository for performing CRUD operations on `Keys` entities.
pub(crate) struct KeyRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for KeyRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl KeyRepository {
    /// Insert a key within a transaction.
    pub(crate) async fn insert_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        key: &Key,
    ) -> Result<u32, sqlx::Error> {
        let value = key.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO keys (value, expire_at,mac) VALUES (?, ?,?)",
            value,
            key.expire_at,
            key.mac
        )
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }

    /// Delete a key by MAC within a transaction.
    pub(crate) async fn delete_by_mac_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        mac: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM keys WHERE mac = ?", mac)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_by_mac(&self, mac: &str) -> Result<Option<Key>, sqlx::Error> {
        let now = chrono::Utc::now();
        let result = sqlx::query!(
            "SELECT id, value, expire_at FROM keys WHERE mac = ? AND expire_at > ?",
            mac,
            now
        )
        .fetch_optional(&*self.pool)
        .await?;
        if let Some(record) = result {
            let id = record.id.ok_or_else(|| sqlx::Error::RowNotFound)? as u32;
            Ok(Some(Key {
                id,
                mac: mac.to_string(),
                value: record.value,
                expire_at: DateTime::<Utc>::from_naive_utc_and_offset(record.expire_at, Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all valid (non-expired) keys.
    pub(crate) async fn get_all(&self) -> Result<Vec<Key>, sqlx::Error> {
        let now = chrono::Utc::now();

        // Utiliser query_as au lieu de query! pour éviter les problèmes de cache offline
        let results: Vec<KeyRow> =
            sqlx::query_as("SELECT id, value, expire_at, mac FROM keys WHERE expire_at > ?")
                .bind(now)
                .fetch_all(&*self.pool)
                .await?;

        let keys = results
            .into_iter()
            .map(|row| Key {
                id: row.id as u32,
                mac: row.mac,
                value: row.value,
                expire_at: DateTime::<Utc>::from_naive_utc_and_offset(row.expire_at, Utc),
            })
            .collect();

        Ok(keys)
    }
}

/// Row structure for query_as
#[derive(sqlx::FromRow)]
struct KeyRow {
    id: i64,
    value: Vec<u8>,
    expire_at: chrono::NaiveDateTime,
    mac: String,
}

#[cfg(test)]
mod tests;
