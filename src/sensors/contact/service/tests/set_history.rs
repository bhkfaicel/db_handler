use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    history::sensors::model::fields::{TimeUnit, VibrationCount},
    sensors::{
        contact::{error::Result, service::ContactSensorService},
        model::{ProductType, Status},
        service::SensorService,
    },
};
use chrono::Utc;

/// Helper function to convert anyhow::Error to ContactSensorError
fn to_contact_sensor_error(e: anyhow::Error) -> crate::sensors::contact::error::ContactSensorError {
    crate::sensors::contact::error::ContactSensorError::operation(e.to_string())
}

/// Helper function to convert SensorError to ContactSensorError
fn sensor_error_to_contact_error(
    e: crate::sensors::error::SensorError,
) -> crate::sensors::contact::error::ContactSensorError {
    crate::sensors::contact::error::ContactSensorError::operation(e.to_string())
}

/// Integration tests for `ContactSensorService::set_history` (primary DB + InfluxDB).
///
/// These require a live InfluxDB instance matching `config.test.json`.
/// CI runs `cargo test` without Docker; run them locally with:
///   `cargo test -- --ignored set_history`
/// or `scripts/test-docker.sh`.
#[tokio::test]
#[ignore = "requires InfluxDB; run with cargo test -- --ignored or scripts/test-docker.sh"]
async fn should_record_history_without_vibration_count() -> Result<()> {
    let connection = Connection::new().await.map_err(to_contact_sensor_error)?;
    let service = ContactSensorService::new(connection.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let _id = sensor_service
        .add("00:1B:44:11:3A:F0")
        .await
        .map_err(sensor_error_to_contact_error)?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:F0",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(sensor_error_to_contact_error)?;

    // Retrieve the contact sensor (initially is_open should be None)
    let sensor = service
        .get_by_mac("00:1B:44:11:3A:F0")
        .await?
        .expect("Sensor should exist");

    // Verify initial state - is_open should be None for a new sensor
    assert_eq!(sensor.is_open(), None, "Initial is_open should be None");

    // Record history with is_open = true
    let result = service.set_history(&sensor, true, None, None).await;
    assert!(result.is_ok(), "set_history should succeed");

    // Retrieve the sensor again to verify is_open was updated in primary DB
    let updated_sensor = service
        .get_by_mac("00:1B:44:11:3A:F0")
        .await?
        .expect("Sensor should still exist");

    // Verify that is_open was updated to true in the primary database
    assert_eq!(
        updated_sensor.is_open(),
        Some(true),
        "is_open should be updated to true in primary database"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires InfluxDB; run with cargo test -- --ignored or scripts/test-docker.sh"]
async fn should_record_history_with_vibration_count() -> Result<()> {
    let connection = Connection::new().await.map_err(to_contact_sensor_error)?;
    let service = ContactSensorService::new(connection.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let _id = sensor_service
        .add("00:1B:44:11:3A:F1")
        .await
        .map_err(sensor_error_to_contact_error)?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:F1",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(sensor_error_to_contact_error)?;

    // Retrieve the contact sensor (initially is_open should be None)
    let sensor = service
        .get_by_mac("00:1B:44:11:3A:F1")
        .await?
        .expect("Sensor should exist");

    // Verify initial state - is_open should be None for a new sensor
    assert_eq!(sensor.is_open(), None, "Initial is_open should be None");

    // Record history with is_open = false and vibration count
    let vibration_count = Some(VibrationCount {
        count: 5,
        time_window: 10,
        unit: TimeUnit::Seconds,
    });

    let result = service
        .set_history(&sensor, false, vibration_count, None)
        .await;
    assert!(result.is_ok(), "set_history should succeed");

    // Retrieve the sensor again to verify is_open was updated in primary DB
    let updated_sensor = service
        .get_by_mac("00:1B:44:11:3A:F1")
        .await?
        .expect("Sensor should still exist");

    // Verify that is_open was updated to false in the primary database
    assert_eq!(
        updated_sensor.is_open(),
        Some(false),
        "is_open should be updated to false in primary database"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires InfluxDB; run with cargo test -- --ignored or scripts/test-docker.sh"]
async fn should_record_history_with_sensor_timestamp() -> Result<()> {
    let connection = Connection::new().await.map_err(to_contact_sensor_error)?;
    let service = ContactSensorService::new(connection.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let _id = sensor_service
        .add("00:1B:44:11:3A:F2")
        .await
        .map_err(sensor_error_to_contact_error)?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:F2",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(sensor_error_to_contact_error)?;

    // Retrieve the contact sensor (initially is_open should be None)
    let sensor = service
        .get_by_mac("00:1B:44:11:3A:F2")
        .await?
        .expect("Sensor should exist");

    // Verify initial state - is_open should be None for a new sensor
    assert_eq!(sensor.is_open(), None, "Initial is_open should be None");

    // Record history with is_open = true and sensor timestamp
    let sensor_timestamp = Some(Utc::now());

    let result = service
        .set_history(&sensor, true, None, sensor_timestamp)
        .await;

    assert!(result.is_ok(), "set_history should succeed");

    // Retrieve the sensor again to verify is_open was updated in primary DB
    let updated_sensor = service
        .get_by_mac("00:1B:44:11:3A:F2")
        .await?
        .expect("Sensor should still exist");

    // Verify that is_open was updated to true in the primary database
    assert_eq!(
        updated_sensor.is_open(),
        Some(true),
        "is_open should be updated to true in primary database"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires InfluxDB; run with cargo test -- --ignored or scripts/test-docker.sh"]
async fn should_update_is_open_when_value_already_exists() -> Result<()> {
    let connection = Connection::new().await.map_err(to_contact_sensor_error)?;
    let service = ContactSensorService::new(connection.clone());

    // Create a sensor and convert it to a contact sensor
    let sensor_service = SensorService::new(connection.clone());
    let _id = sensor_service
        .add("00:1B:44:11:3A:F3")
        .await
        .map_err(sensor_error_to_contact_error)?;

    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:F3",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(sensor_error_to_contact_error)?;

    // Retrieve the contact sensor (initially is_open should be None)
    let sensor = service
        .get_by_mac("00:1B:44:11:3A:F3")
        .await?
        .expect("Sensor should exist");

    // Verify initial state - is_open should be None for a new sensor
    assert_eq!(sensor.is_open(), None, "Initial is_open should be None");

    // First, set is_open to true
    let result = service.set_history(&sensor, true, None, None).await;
    assert!(result.is_ok(), "First set_history should succeed");

    // Retrieve and verify is_open is now true
    let sensor_after_first_update = service
        .get_by_mac("00:1B:44:11:3A:F3")
        .await?
        .expect("Sensor should still exist");
    assert_eq!(
        sensor_after_first_update.is_open(),
        Some(true),
        "is_open should be true after first update"
    );

    // Now, change is_open to false (simulating state change)
    let result = service
        .set_history(&sensor_after_first_update, false, None, None)
        .await;
    assert!(result.is_ok(), "Second set_history should succeed");

    // Retrieve again and verify is_open was updated to false
    let sensor_after_second_update = service
        .get_by_mac("00:1B:44:11:3A:F3")
        .await?
        .expect("Sensor should still exist");

    assert_eq!(
        sensor_after_second_update.is_open(),
        Some(false),
        "is_open should be updated to false in primary database after state change"
    );
    Ok(())
}
