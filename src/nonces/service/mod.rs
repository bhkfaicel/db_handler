use std::sync::Arc;

use crate::db::connection::Connection;
use crate::db::primary::error::CommonError;
use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::Service;
use crate::nonces::{
    error::{NonceError, Result},
    model::Nonce,
    repository::NonceRepository,
};

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
    /// Records a nonce in the anti-replay ledger.
    ///
    /// # Ledger contract
    ///
    /// A nonce value is burned GLOBALLY once accepted: the same value is
    /// refused afterwards no matter which sensor presents it. Detection is
    /// enforced by the database — the `UNIQUE(value)` constraint rejects
    /// the duplicate inside the INSERT itself, so concurrent writers get
    /// exactly one acceptance with no SELECT-then-INSERT probe (a racy
    /// check that `SQLITE_BUSY` under contention could defeat). Accepted
    /// rows are never deleted on verification; they are retained until the
    /// presenting sensor's key row is deleted at key rotation, whose
    /// `ON DELETE CASCADE` removes that sensor's nonces, so ledger growth
    /// is bounded by the key-rotation window. With 96-bit values, two
    /// distinct sensors independently generating the same nonce has
    /// probability about n²/2⁹⁷ (roughly 6e-18 at one million nonces per
    /// window), so the global scope never rejects legitimate traffic in
    /// practice.
    ///
    /// # Nonce entropy contract
    ///
    /// `value` is type-enforced as `[u8; 12]` (96 bits) and MUST come from
    /// a CSPRNG, freshly generated for every authenticated message; a
    /// reused value makes every subsequent message a replay by definition.
    ///
    /// # MAC format contract
    ///
    /// `mac` must be six hexadecimal pairs separated by `:` (case
    /// -insensitive, e.g. `AA:BB:26:06:19:91`); any other value is
    /// rejected as `NonceError::InvalidMacFormat` before any database
    /// work happens.
    ///
    /// # Arguments
    ///
    /// * `nonce` - array of 12 bytes containing the nonce value.
    /// * `mac` - sensor's mac.
    ///
    /// # Returns
    ///
    /// * `Ok(Nonce)` - The stored record (ID filled in).
    /// * `Err(NonceError::ReplayDetected)` - The value was already accepted
    ///   before, whatever the sensor: the replay is refused, the original
    ///   ledger row is left untouched.
    /// * `Err(NonceError::InvalidMacFormat)` - The MAC is malformed.
    /// * `Err(NonceError)` - Any other error (e.g. a MAC with no key row
    ///   fails the foreign key and keeps the structured
    ///   `CommonError::Database` payload).
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
        // Trust boundary: a malformed MAC is refused before any database
        // work, so no ill-formed address can enter the anti-replay ledger.
        if !crate::common::helpers::mac::is_valid(mac) {
            return Err(NonceError::InvalidMacFormat(mac.to_string()));
        }

        let mut nonce = Nonce {
            id: 0,
            value: nonce.to_vec(),
            mac: mac.to_string(),
        };
        // Replay detection is DB-enforced: the global UNIQUE(value)
        // constraint rejects an already-burned value inside the INSERT, so
        // no SELECT-then-INSERT probe is used (that racy two-step check
        // could be defeated by concurrent writers and SQLITE_BUSY).
        match self.repository.insert(&nonce).await {
            Ok(id) => {
                nonce.id = id;
                Ok(nonce)
            }
            // The ledger's own constraint means the value was already
            // accepted, whatever the presenting sensor: report the typed
            // replay outcome. Any other unique violation (none exists on
            // this table today) would keep its structured payload below.
            Err(CommonError::UniqueViolation { constraint }) if constraint == "nonces.value" => {
                Err(NonceError::ReplayDetected)
            }
            // Everything else (foreign-key failure on an unknown MAC,
            // generic database errors) keeps its `CommonError` payload.
            Err(e) => Err(e.into()),
        }
    }
}
#[cfg(test)]
mod tests;
