use chrono::{DateTime, Duration, Utc};
use sqlx::Transaction;
use std::sync::Arc;

use crate::db::connection::Connection;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::{map_sqlx_to_common_error, Service};
use crate::history::keys::{
    error::{HistoricalKeyError, Result},
    model::HistoricalKey,
    repository::HistoricalKeyRepository,
};
/// Helper function to convert `sqlx::Error` to `HistoricalKeyError`.
///
/// This function uses the centralized `map_sqlx_to_common_error` function
/// to convert database errors to `HistoricalKeyError`.
fn map_sqlx_error(e: sqlx::Error) -> HistoricalKeyError {
    map_sqlx_to_common_error(e).into()
}

pub struct HistoricalKeyService {
    pub(crate) repository: Arc<HistoricalKeyRepository>,
}
impl Service for HistoricalKeyService {
    fn new(connection: Connection) -> Self {
        let repository = HistoricalKeyRepository::new(connection.primary_db_client());
        Self {
            repository: Arc::new(repository),
        }
    }
}
impl HistoricalKeyService {
    /// Inserts a new historical_key into the database.
    ///
    /// # Arguments
    ///
    /// * `value` - array of 32 bytes containing the key.
    ///
    /// # Returns
    ///
    /// * `Ok(HistoricalKey)` - The newly inserted historical key.
    /// * `Err(HistoricalKeyError::Common(CommonError::Database(_)))` - If a database error occurs.
    ///
    /// # Example
    ///
    ///  no_run
    /// use db_handler::prelude::*;
    /// use db_handler::history::keys::service::HistoricalKeyService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = HistoricalKeyService::new(connection);
    ///     let key = [26; 32];
    ///     let historical_key = service.add(key).await?;
    ///     println!("Inserted historical key : {:?}", historical_key);
    ///     Ok(())
    ///  }
    ///

    pub async fn add(&self, key: [u8; 32]) -> Result<HistoricalKey> {
        let mut expire_at: DateTime<Utc> = Utc::now();
        expire_at = expire_at + Duration::days(30);
        let mut historical_key = HistoricalKey {
            id: 0,
            value: key.to_vec(),
            expire_at: expire_at,
        };
        let id = self
            .repository
            .insert(&historical_key)
            .await
            .map_err(map_sqlx_error)?;
        historical_key.id = id;
        Ok(historical_key)
    }

    /// Inserts a new historical_key into the database within a transaction.
    ///
    /// # Arguments
    ///
    /// * `value` - array of 32 bytes containing the key.
    ///
    /// # Returns
    ///
    /// * `Ok(HistoricalKey)` - The newly inserted historical key.
    /// * `Err(HistoricalKeyError::Common(CommonError::Database(_)))` - If a database error occurs.
    ///
    /// # Example
    ///
    ///  no_run
    /// use db_handler::prelude::*;
    /// use db_handler::history::keys::service::HistoricalKeyService;
    /// use db_handler::db::connection::Connection;
    /// use sqlx::Transaction;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = HistoricalKeyService::new(connection);
    ///     let key = [26; 32];
    ///     let mut tx = connection.primary_db_client().begin().await?;
    ///     let historical_key = service.add_with_transaction(&mut tx, key).await?;
    ///     tx.commit().await?;
    ///     println!("Inserted historical key : {:?}", historical_key);
    ///     Ok(())
    ///  }
    ///

    pub(crate) async fn add_with_transaction(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        key: [u8; 32],
    ) -> Result<HistoricalKey> {
        let mut expire_at: DateTime<Utc> = Utc::now();
        expire_at = expire_at + Duration::days(30);
        let mut historical_key = HistoricalKey {
            id: 0,
            value: key.to_vec(),
            expire_at: expire_at,
        };
        let id = self
            .repository
            .insert_with_transaction(tx, &historical_key)
            .await
            .map_err(map_sqlx_error)?;
        historical_key.id = id;
        Ok(historical_key)
    }
}
#[cfg(test)]
mod tests;
