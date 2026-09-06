use super::*;
use crate::db::connection::Connection;
use crate::db::primary::error::CommonError;
use crate::db::primary::service::Service;
use crate::keys::service::KeyService;
use crate::nonces::error::NonceError;
use crate::sensors::service::SensorService;
// Explicit anyhow Result: the service module re-exports the nonce domain
// `Result` alias through `use super::*`, which must not shadow the tests'
// return type.
use anyhow::{Ok, Result};

#[tokio::test]
async fn test_add_nonce() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let sensor_service = SensorService::new(connection.clone());
    let service = NonceService::new(connection.clone());
    let key_service = KeyService::new(connection);

    let mac = "AA:BB:26:06:19:91";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = [26; 12];
    let key = [26; 32];
    let _ = key_service
        .add(key, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = service
        .add(nonce, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let created_nonce = sqlx::query_as::<_, Nonce>(
        r#"
        SELECT * FROM nonces WHERE id = ?
        "#,
    )
    .bind(nonce.id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created historical key");
    assert!(nonce.id > 0);
    assert_eq!(created_nonce, nonce);
    Ok(())
}

/// Non-regression: inserting the same nonce (same value AND same mac) twice
/// through the service must surface the structured unique violation carried
/// by the transparent `Common` variant of `NonceError`. The service maps the
/// repository's `CommonError` without message sniffing; the semantics are
/// the ones the duplicate path produced before the repository switched to
/// `CommonError` (a database-level unique-violation error, no dedicated
/// replay-detection variant at the architecture layer).
#[tokio::test]
async fn add_duplicate_nonce_yields_unique_violation() -> Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let service = NonceService::new(connection.clone());
    let key_service = KeyService::new(connection);

    let mac = "AA:BB:26:06:19:91";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let _ = key_service
        .add([26; 32], mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let nonce = [26; 12];
    service
        .add(nonce, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // The second insert of the identical (value, mac) pair violates the
    // UNIQUE constraints declared on the nonces table; the typed behavior
    // must stay NonceError::Common(CommonError::UniqueViolation) with the
    // constraint identity SQLite reports for the violated index.
    let duplicate = service.add(nonce, mac).await;
    match duplicate {
        Err(NonceError::Common(CommonError::UniqueViolation { constraint }))
            if constraint == "nonces.value" || constraint == "nonces.value, nonces.mac" => {}
        other => panic!("Expected UniqueViolation, got {:?}", other),
    }
    Ok(())
}
