use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    sensors::{model::Status, service::SensorService},
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_return_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mac = "00:1B:44:11:3A:B8";
    let id = service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = service
        .get_by_mac(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_some());
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.mac(), mac);
    assert_eq!(retrieved_sensor.status(), &Status::InProgress);
    assert_eq!(retrieved_sensor.id(), id);
    Ok(())
}

#[tokio::test]
async fn should_return_none() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mac = "00:1B:44:11:3A:B8";
    service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = service
        .get_by_mac("00:1B:44:11:3A:B8t")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
