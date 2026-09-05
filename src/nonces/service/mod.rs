use std::sync::Arc;

use crate::db::connection::Connection;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::{map_sqlx_to_common_error, Service};
use crate::nonces::{
    error::{NonceError, Result},
    model::Nonce,
    repository::NonceRepository,
};

/// Helper function to convert `sqlx::Error` to `NonceError`.
///
/// This function uses the centralized `map_sqlx_to_common_error` function
/// to convert database errors to `NonceError`.
fn map_sqlx_error(e: sqlx::Error) -> NonceError {
    map_sqlx_to_common_error(e).into()
}

pub struct NonceService {
    pub(crate) repository: Arc<NonceRepository>,
}
impl Service for NonceService {
    fn new(connection: Connection) -> Self {
        let repository = NonceRepository::new(connection.primary_db_client());
        Self {
            repository: Arc::new(repository),
        }
    }
}
impl NonceService {
    /// Inserts a new nonce into the database.
    ///
    /// # Arguments
    ///
    /// * `value` - array of 12 bytes containing the key.
    /// * `mac` - sensor's mac.
    ///
    /// # Returns
    ///
    /// * `Ok(Key)` - The ID of the newly inserted nonce.
    /// * `Err(sqlx::Error)` - If an error occurs during insertion.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::prelude::*;
    /// use db_handler::nonces::service::NonceService;
    /// use db_handler::keys::service::KeyService;
    /// use db_handler::db::connection::Connection;
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let connection = Connection::new().await?;
    ///     let service = NonceService::new(connection.clone());
    ///     let key_service = KeyService::new(connection);
    ///     let nonce = [26; 12];
    ///     let key = [26; 32];
    ///     let mac= "AA:BB:26:06:19:91";
    ///     let _ = key_service.add(key,mac).await?;
    ///     let nonce = service.add(nonce,mac).await?;
    ///     println!("Inserted nonce : {:?}", nonce);
    ///     Ok(())
    ///  }
    /// ```
    pub async fn add(&self, nonce: [u8; 12], mac: &str) -> Result<Nonce> {
        let mut nonce = Nonce {
            id: 0,
            value: nonce.to_vec(),
            mac: mac.to_string(),
        };
        let id = self
            .repository
            .insert(&nonce)
            .await
            .map_err(map_sqlx_error)?;
        nonce.id = id;
        Ok(nonce)
    }
}
#[cfg(test)]
mod tests;
