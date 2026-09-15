use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    sensors::{model::Status, service::SensorService},
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_return_raw_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mac = "00:1B:44:11:3A:B9";
    let id = service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = service
        .get_by_id_raw(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_some());
    // The raw accessor returns the plain `Sensor` row as stored by `add`:
    // generated id, seeded MAC, default InProgress status, no sensor type.
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id, id);
    assert_eq!(retrieved_sensor.mac, mac);
    assert_eq!(retrieved_sensor.status, Status::InProgress);
    assert_eq!(retrieved_sensor.sensor_type, None);
    Ok(())
}

#[tokio::test]
async fn should_return_none() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mac = "00:1B:44:11:3A:B9";
    let id = service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    // The only existing row has `id`; the next id must not resolve.
    let retrieved_sensor = service
        .get_by_id_raw(id + 1)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
