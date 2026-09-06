use crate::db::primary::error::Result;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::map_sqlx_to_common_error;
use crate::nonces::model::Nonce;
use sqlx::SqlitePool;
use std::sync::Arc;
/// Repository for performing CRUD operations on `Nonces` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so unique violations surface as the
/// structured `CommonError::UniqueViolation` variant instead of requiring
/// callers to inspect error messages.
pub(crate) struct NonceRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for NonceRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl NonceRepository {
    /// Insert a nonce.
    ///
    /// Duplicate nonces (the `value` column and the `(value, mac)` pair are
    /// both declared UNIQUE in the schema) surface as
    /// `CommonError::UniqueViolation`; foreign-key failures (unknown MAC)
    /// keep the underlying sqlx error inside the generic `Database` variant.
    pub(crate) async fn insert(&self, nonce: &Nonce) -> Result<u32> {
        let value = nonce.value.to_vec();
        let result = sqlx::query!(
            "INSERT INTO nonces (value, mac) VALUES (?, ?)",
            value,
            nonce.mac
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }
}

#[cfg(test)]
mod tests;
