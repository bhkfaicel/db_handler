use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    sensors::{
        contact::service::ContactSensorService,
        model::{ProductType, Status, TypedSensorTrait},
        service::SensorService,
    },
};
use anyhow::{Ok, Result};

/// Tests for retrieving a contact sensor by MAC address through the service layer.
#[tokio::test]
async fn should_return_contact_sensor_by_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let service = ContactSensorService::new(connection.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:E0")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:E0",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Retrieve the contact sensor by MAC
    let retrieved_sensor = service.get_by_mac("00:1B:44:11:3A:E0").await?;
    assert!(retrieved_sensor.is_some());
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:E0");
    assert_eq!(retrieved_sensor.status(), &Status::Installed);
    Ok(())
}

#[tokio::test]
async fn should_return_none_when_mac_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let service = ContactSensorService::new(connection);

    // Try to retrieve a non-existent sensor
    let retrieved_sensor = service.get_by_mac("00:00:00:00:00:00").await?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
