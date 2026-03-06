use crate::{
    db::connection::Connection,
    db::primary::{repository::PrimaryDbRepository, service::Service},
    sensors::{
        contact::repository::ContactSensorRepository,
        model::{ProductType, Status, TypedSensorTrait},
        service::SensorService,
    },
};
use anyhow::{Ok, Result};

/// Tests for retrieving a contact sensor by MAC address.
///
/// This test verifies that:
/// - A contact sensor can be retrieved by its MAC address
/// - The retrieved sensor has the correct properties
/// - None is returned when the sensor doesn't exist
#[tokio::test]
async fn should_return_contact_sensor_by_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:C0")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:C0",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Retrieve the contact sensor by MAC
    let retrieved_sensor = repository.get_by_mac("00:1B:44:11:3A:C0").await?;
    assert!(retrieved_sensor.is_some());
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:C0");
    assert_eq!(retrieved_sensor.status(), &Status::Installed);
    Ok(())
}

#[tokio::test]
async fn should_return_none_when_mac_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool);

    // Try to retrieve a non-existent sensor
    let retrieved_sensor = repository.get_by_mac("00:00:00:00:00:00").await?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}

#[tokio::test]
async fn should_return_none_when_sensor_is_not_contact_sensor_by_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());

    // Create a sensor but don't convert it to contact sensor
    let sensor_service = SensorService::new(connection.clone());
    sensor_service
        .add("00:1B:44:11:3A:C1")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Try to retrieve as contact sensor (should return None since it's not in open_close_sensors)
    let retrieved_sensor = repository.get_by_mac("00:1B:44:11:3A:C1").await?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
