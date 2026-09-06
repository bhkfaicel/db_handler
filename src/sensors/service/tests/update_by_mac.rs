use crate::{
    db::{connection::Connection, primary::service::Service},
    sensors::{
        error::SensorError,
        model::{ProductType, Status, TypedSensor},
        service::SensorService,
    },
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_update() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B9")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let updated_sensors_result = service
        .update_by_mac(
            "00:1B:44:11:3A:B9",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert_eq!(updated_sensors_result, 1);
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:B9");
    assert_eq!(retrieved_sensor.status(), &Status::Installed);
    println!("---------------------------------: {:?}", retrieved_sensor);
    assert!(matches!(retrieved_sensor, TypedSensor::ContactSensor(_)));
    Ok(())
}

#[tokio::test]
async fn should_return_0_mac_does_not_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B9")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let updated_sensors_result = service
        .update_by_mac(
            "00:1B:44:11:3A:B",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert_eq!(updated_sensors_result, 0);
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:B9");
    assert_eq!(retrieved_sensor.status(), &Status::InProgress);
    assert!(matches!(retrieved_sensor, TypedSensor::Unknown(_)));
    Ok(())
}

#[tokio::test]
async fn should_return_error_no_fields() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B9")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let updated_sensors_result = service.update_by_mac("00:1B:44:11:3A:B9", None, None).await;
    // Typed non-regression assertion: an empty update is rejected with the
    // dedicated variant before any SQL is sent.
    assert!(matches!(
        updated_sensors_result,
        Err(SensorError::NoFieldsForUpdate)
    ));
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:B9");
    assert_eq!(retrieved_sensor.status(), &Status::InProgress);
    assert!(matches!(retrieved_sensor, TypedSensor::Unknown(_)));
    Ok(())
}
#[tokio::test]
async fn should_return_error_type_is_in_progress() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B9")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let updated_sensors_result = service
        .update_by_mac(
            "00:1B:44:11:3A:B9",
            Some(Status::InProgress),
            Some(ProductType::ContactSensor),
        )
        .await;
    // Typed non-regression assertion: reverting a status to `InProgress` is
    // rejected with the dedicated variant before any SQL is sent.
    assert!(matches!(
        updated_sensors_result,
        Err(SensorError::CannotUpdateToInProgress)
    ));
    let retrieved_sensor = service
        .get_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.id(), id);
    assert_eq!(retrieved_sensor.mac(), "00:1B:44:11:3A:B9");
    assert_eq!(retrieved_sensor.status(), &Status::InProgress);
    assert!(matches!(retrieved_sensor, TypedSensor::Unknown(_)));
    Ok(())
}
