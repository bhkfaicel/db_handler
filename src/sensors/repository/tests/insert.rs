use crate::{
    db::connection::Connection,
    db::primary::repository::PrimaryDbRepository,
    sensors::{
        model::{Sensor, Status},
        repository::SensorRepository,
    },
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_insert_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool.clone());
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B7".to_string(), Status::Installed);
    let id = repository.insert(&sensor).await?;
    let created_sensor = sqlx::query_as::<_, Sensor>(
        r#"
        SELECT * FROM sensors WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created sensor");
    assert!(id > 0);
    assert_eq!(created_sensor.id, id);
    assert_eq!(created_sensor.mac, "00:1B:44:11:3A:B7".to_string());
    assert_eq!(created_sensor.status, Status::Installed);
    Ok(())
}

#[tokio::test]
async fn should_panic_because_mac_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool.clone());
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B7".to_string(), Status::Installed);
    let id = repository.insert(&sensor).await?;
    let created_sensor = sqlx::query_as::<_, Sensor>(
        r#"
        SELECT * FROM sensors WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created sensor");
    assert!(id > 0);
    assert_eq!(created_sensor.mac, "00:1B:44:11:3A:B7".to_string());
    let insert_result = repository.insert(&sensor).await;
    if let Err(err) = insert_result {
        // Extract the SQLx error from `anyhow::Error`
        if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
            if let Some(db_err) = sqlx_err.as_database_error() {
                let error_message = db_err.message();
                let error_code = db_err.code().map(|c| c.to_string()).unwrap_or_default();
                // Verify that it's a UNIQUE constraint violation
                assert!(
                    error_message.contains("UNIQUE constraint failed: sensors.mac"),
                    "UNIQUE constraint failed: sensors.mac got: {}",
                    error_message
                );
                assert_eq!(error_code, "2067");
            } else {
                panic!("Unexpected non-database error: {:?}", sqlx_err);
            }
        } else {
            panic!("Unexpected non-SQLx error: {:?}", err);
        }
    } else {
        panic!("Expected an error, but insert succeeded");
    }
    Ok(())
}
