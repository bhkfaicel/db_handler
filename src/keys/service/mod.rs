use chrono::{Duration, Utc};
use log::warn;
use std::sync::Arc;

use crate::db::connection::Connection;
use crate::db::primary::error::CommonError;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::{map_sqlx_to_common_error, Service};
use crate::history::keys::service::HistoricalKeyService;
use crate::keys::{
    error::{KeyError, Result},
    model::Key,
    repository::KeyRepository,
};

/// Helper function to convert `sqlx::Error` to `KeyError`.
///
/// This function uses the centralized `map_sqlx_to_common_error` function
/// to convert database errors to `KeyError`.
fn map_sqlx_error(e: sqlx::Error) -> KeyError {
    map_sqlx_to_common_error(e).into()
}

/// Default expiration time for keys in hours.
const DEFAULT_KEY_EXPIRATION_HOURS: i64 = 1;

pub struct KeyService {
    repository: Arc<KeyRepository>,
    connection: Connection,
}
impl Service for KeyService {
    fn new(connection: Connection) -> Self {
        let repository = KeyRepository::new(connection.primary_db_client().clone());
        Self {
            repository: Arc::new(repository),
            connection,
        }
    }
}
impl KeyService {
    /// Inserts a new key into the database.
    ///
    /// # Arguments
    ///
    /// * `value` - array of 32 bytes containing the key.
    /// * `mac` - string
    ///
    /// # Returns
    ///
    /// * `Ok(Key)` - The newly inserted key.
    /// * `Err(KeyError)` - If an error occurs during insertion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::keys::service::KeyService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = KeyService::new(connection);
    ///     let key = [26; 32];
    ///     let mac = "AA:BB:26:06:19:91";
    ///     let key = service.add(key, mac).await?;
    ///     println!("Inserted key : {:?}", key);
    ///     Ok(())
    ///  }
    /// ```
    pub async fn add(&self, key: [u8; 32], mac: &str) -> Result<Key> {
        // Start a transaction to ensure atomicity
        let mut tx = self
            .connection
            .primary_db_client()
            .begin()
            .await
            .map_err(map_sqlx_error)?;

        let historical_key_service = HistoricalKeyService::new(self.connection.clone());
        historical_key_service
            .add_with_transaction(&mut tx, key)
            .await?;

        // Prepare the key
        let expire_at = Utc::now() + Duration::hours(DEFAULT_KEY_EXPIRATION_HOURS);
        let mut key = Key {
            id: 0,
            value: key.to_vec(),
            expire_at,
            mac: mac.to_string(),
        };

        // Try to delete existing key for this MAC (it's expected if none exists)
        // Log but don't fail if deletion fails
        if let Err(e) = self
            .repository
            .delete_by_mac_with_transaction(&mut tx, mac)
            .await
        {
            warn!(
                "Failed to delete existing key for MAC {} (this is expected if no key exists): {}",
                mac, e
            );
        }

        // Insert the new key within transaction
        let insert_result = self.repository.insert_with_transaction(&mut tx, &key).await;

        let id = match insert_result {
            Ok(id) => id,
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    warn!(
                        "Failed to rollback transaction after key insertion error: {}",
                        rollback_err
                    );
                }
                return Err(KeyError::Common(CommonError::Database(e)));
            }
        };

        // Commit the transaction
        if let Err(e) = tx.commit().await {
            return Err(KeyError::Common(CommonError::Database(e)));
        }

        key.id = id;
        Ok(key)
    }
    /// Get key by mac.
    ///
    /// # Arguments
    ///
    /// * `mac` - string
    ///
    /// # Returns
    ///
    /// * `Ok(Key)` - If a key with the specified mac exists.
    /// * `Err(KeyError::NotFound)` - If no key with the specified mac is found.
    /// * `Err(KeyError::Database)` - If a database error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::keys::service::KeyService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = KeyService::new(connection);
    ///     let mac = "AA:BB:26:06:19:91";
    ///     let key = service.get_by_mac(mac).await?;
    ///     println!("key : {:?}", key);
    ///     Ok(())
    ///  }
    /// ```
    pub async fn get_by_mac(&self, mac: &str) -> Result<Key> {
        self.repository
            .get_by_mac(mac)
            .await
            .map_err(map_sqlx_error)?
            .ok_or_else(|| KeyError::not_found(mac))
    }

    /// Get all valid (non-expired) keys from the database.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Key>)` - List of all valid keys.
    /// * `Err(KeyError)` - If a database error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::keys::service::KeyService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = KeyService::new(connection);
    ///     let keys = service.get_all().await?;
    ///     println!("Found {} keys", keys.len());
    ///     Ok(())
    ///  }
    /// ```
    pub async fn get_all(&self) -> Result<Vec<Key>> {
        self.repository.get_all().await.map_err(map_sqlx_error)
    }
}
#[cfg(test)]
mod tests;
