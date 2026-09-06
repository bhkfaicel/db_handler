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
