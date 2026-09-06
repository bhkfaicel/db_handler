use crate::{
    db::connection::Connection,
    db::primary::repository::PrimaryDbRepository,
    sensors::{
        model::{ProductType, Sensor, Status},
        repository::SensorRepository,
    },
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_update() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool.clone());
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B7".to_string(), Status::InProgress);
    let id = repository.insert(&sensor).await?;
    let affected_rows = repository
        .update_by_mac(
            "00:1B:44:11:3A:B7",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await?;
    assert_eq!(affected_rows, 1);
    let updated_sensor = sqlx::query_as::<_, Sensor>(
        r#"
        SELECT * FROM sensors WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created sensor");
    assert_eq!(updated_sensor.id, id);
    assert_eq!(updated_sensor.mac, "00:1B:44:11:3A:B7".to_string());
    assert_eq!(updated_sensor.status, Status::Installed);
    assert_eq!(
        updated_sensor.sensor_type.unwrap(),
        ProductType::ContactSensor
    );
    Ok(())
}

#[tokio::test]
async fn shouldnt_update_beacause_mac_does_not_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool.clone());
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B7".to_string(), Status::InProgress);
    let id = repository.insert(&sensor).await?;
    let affected_rows = repository
        .update_by_mac(
            "mac does not exists",
            Some(Status::Installed),
            Some(ProductType::ContactSensor),
        )
        .await?;
    assert_eq!(affected_rows, 0);
    let updated_sensor = sqlx::query_as::<_, Sensor>(
        r#"
        SELECT * FROM sensors WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created sensor");
    assert_eq!(updated_sensor.id, id);
    assert_eq!(updated_sensor.mac, "00:1B:44:11:3A:B7".to_string());
    assert_eq!(updated_sensor.status, Status::InProgress);
    assert!(updated_sensor.sensor_type.is_none());
    Ok(())
}

#[tokio::test]
async fn should_update_only_type() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool.clone());
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B7".to_string(), Status::InProgress);
    let id = repository.insert(&sensor).await?;
    let _ = repository
        .update_by_mac("00:1B:44:11:3A:B7", None, Some(ProductType::ContactSensor))
        .await?;
    let updated_sensor = sqlx::query_as::<_, Sensor>(
        r#"
        SELECT * FROM sensors WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created sensor");
    println!("{:?}", updated_sensor);
    assert_eq!(updated_sensor.id, id);
    assert_eq!(updated_sensor.mac, "00:1B:44:11:3A:B7".to_string());
    assert_eq!(updated_sensor.status, Status::InProgress);
    assert_eq!(
        updated_sensor.sensor_type.unwrap(),
        ProductType::ContactSensor
    );
    Ok(())
}

// NOTE: the business rules (no update to `InProgress`, at least one field
// required) moved out of the repository into `SensorService`, which rejects
// them before any SQL is sent. Their non-regression coverage lives in the
// service tests (`src/sensors/service/tests/update_by_mac.rs`) with typed
// `SensorError` variant assertions, replacing the two former
// message-less `is_err()` repository tests.
