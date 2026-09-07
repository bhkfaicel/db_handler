use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    sensors::{error::SensorError, service::SensorService},
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_insert_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(id > 0);
    Ok(())
}

/// Typed non-regression test: inserting a second sensor with an already
/// registered MAC must yield the dedicated `DuplicateMac` variant carrying
/// the offending MAC, mapped from the structured unique-violation constraint
/// (`sensors.mac`) reported by the repository.
#[tokio::test]
async fn should_reject_duplicate_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let duplicate = service.add("00:1B:44:11:3A:B7").await;
    assert!(
        matches!(&duplicate, Err(SensorError::DuplicateMac(mac)) if mac == "00:1B:44:11:3A:B7"),
        "expected SensorError::DuplicateMac, got: {duplicate:?}"
    );
    Ok(())
}

/// The MAC format contract is enforced at the service's trust boundary:
/// `sensors.mac` roots the foreign-key chain (`keys.mac` ->
/// `nonces.mac`), so a malformed address (line-protocol metacharacters)
/// is refused as `SensorError::InvalidMac` BEFORE any database work and
/// no row can be inserted.
#[tokio::test]
async fn should_reject_invalid_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let service = SensorService::new(connection);

    let malicious = "AA,BB CC=DD:EE:FF";
    let result = service.add(malicious).await;
    assert!(
        matches!(&result, Err(SensorError::InvalidMac(mac)) if mac == malicious),
        "expected SensorError::InvalidMac, got: {result:?}"
    );
    // Nothing may have been written for the refused MAC.
    let stored = sqlx::query("SELECT 1 FROM sensors WHERE mac = ?")
        .bind(malicious)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(
        stored.is_none(),
        "a refused MAC must not be inserted into sensors"
    );
    Ok(())
}
