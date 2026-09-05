use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::{db::primary::repository::PrimaryDbRepository, nonces::model::Nonce};
/// Repository for performing CRUD operations on `Nonces` entities.
pub(crate) struct NonceRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for NonceRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl NonceRepository {
    pub(crate) async fn insert(&self, nonce: &Nonce) -> Result<u32, sqlx::Error> {
        let value = nonce.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO nonces (value, mac) VALUES (?, ?)",
            value,
            nonce.mac
        )
        .execute(&*self.pool)
        .await;
        match result {
            Ok(result) => Ok(result.last_insert_rowid() as u32),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests;
