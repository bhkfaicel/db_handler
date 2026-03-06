use crate::db::primary::repository::PrimaryDbRepository;
use crate::db::primary::service::Service;
use crate::sensors::model::{ProductType, Status, TypedSensorTrait};
use crate::sensors::service::SensorService;
use crate::{db::connection::Connection, sensors::contact::repository::ContactSensorRepository};
use anyhow::{Ok, Result};
use chrono::{NaiveDateTime, Utc};

/// Tests for updating a contact sensor by its MAC address.
///
/// This test verifies that:
/// - A contact sensor can be updated by its MAC address
/// - The updated sensor has the correct properties
/// - The number of rows updated is correct
#[tokio::test]
async fn should_insert_is_open() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:B7",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let affected_rows = repository
        .update_by_mac(id, "00:1B:44:11:3A:B7", true, Utc::now().naive_utc())
        .await?;
    assert_eq!(affected_rows, 1);
    let updated_sensor = repository.get_by_mac("00:1B:44:11:3A:B7").await?;
    assert!(updated_sensor.is_some());
    let updated_sensor = updated_sensor.unwrap();
    assert_eq!(updated_sensor.id(), id);
    assert_eq!(updated_sensor.mac(), "00:1B:44:11:3A:B7");
    assert_eq!(updated_sensor.status(), &Status::Installed);
    assert_eq!(updated_sensor.is_open(), Some(true));
    assert!(updated_sensor.updated_at().is_some());
    assert!(updated_sensor.updated_at().unwrap() != NaiveDateTime::default());
    Ok(())
}

#[tokio::test]
async fn should_insert_and_update_is_open() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:B7",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let affected_rows = repository
        .update_by_mac(id, "00:1B:44:11:3A:B7", true, Utc::now().naive_utc())
        .await?;
    assert_eq!(affected_rows, 1);
    let updated_sensor = repository.get_by_mac("00:1B:44:11:3A:B7").await?;
    assert!(updated_sensor.is_some());
    let updated_sensor = updated_sensor.unwrap();
    assert_eq!(updated_sensor.id(), id);
    assert_eq!(updated_sensor.mac(), "00:1B:44:11:3A:B7");
    assert_eq!(updated_sensor.status(), &Status::Installed);
    assert_eq!(updated_sensor.is_open(), Some(true));
    assert!(updated_sensor.updated_at().is_some());
    assert!(updated_sensor.updated_at().unwrap() != NaiveDateTime::default());
    let affected_rows = repository
        .update_by_mac(id, "00:1B:44:11:3A:B7", false, Utc::now().naive_utc())
        .await?;
    assert_eq!(affected_rows, 1);
    let updated_sensor = repository.get_by_mac("00:1B:44:11:3A:B7").await?;
    assert!(updated_sensor.is_some());
    let updated_sensor = updated_sensor.unwrap();
    assert_eq!(updated_sensor.id(), id);
    assert_eq!(updated_sensor.mac(), "00:1B:44:11:3A:B7");
    assert_eq!(updated_sensor.status(), &Status::Installed);
    assert_eq!(updated_sensor.is_open(), Some(false));
    assert!(updated_sensor.updated_at().is_some());
    assert!(updated_sensor.updated_at().unwrap() != NaiveDateTime::default());
    Ok(())
}

#[tokio::test]
async fn should_not_insert_is_open_because_id_does_not_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:B7",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let affected_rows = repository
        .update_by_mac(id + 1, "00:1B:44:11:3A:B7", true, Utc::now().naive_utc())
        .await;
    assert!(affected_rows.is_err());
    Ok(())
}

#[tokio::test]
async fn should_not_insert_is_open_because_mac_does_not_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = ContactSensorRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let id = sensor_service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .update_by_mac(
            "00:1B:44:11:3A:B7",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let affected_rows = repository
        .update_by_mac(id, "00:1B:44:11:3A:26", true, Utc::now().naive_utc())
        .await;
    assert!(affected_rows.is_err());
    Ok(())
}
