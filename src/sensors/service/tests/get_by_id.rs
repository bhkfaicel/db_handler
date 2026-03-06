use crate::{
    db::{connection::Connection, primary::service::Service},
    sensors::{
        model::{Status, TypedSensor},
        service::SensorService,
    },
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_return_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B8")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_some());
    let retrieved_sensor: TypedSensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:B8");
    assert_eq!(retrieved_sensor.status(), &Status::InProgress);
    assert_eq!(retrieved_sensor.id(), id);
    Ok(())
}

#[tokio::test]
async fn shulde_return_none() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mut id = service
        .add("00:1B:44:11:3A:B8")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    id = id + 1;
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
