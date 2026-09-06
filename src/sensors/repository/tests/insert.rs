use crate::{
    db::{
        connection::Connection,
        primary::{error::CommonError, repository::PrimaryDbRepository},
    },
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
        // The repository reports the duplicate structurally: the central
        // conversion decodes SQLite's message into the constraint name, so
        // the assertion targets the typed variant and the exact constraint
        // SQLite reports for the inline UNIQUE on sensors.mac.
        assert!(
            matches!(
                &err,
                CommonError::UniqueViolation { constraint } if constraint == "sensors.mac"
            ),
            "expected CommonError::UniqueViolation on sensors.mac, got: {err:?}"
        );
    } else {
        panic!("Expected an error, but insert succeeded");
    }
    Ok(())
}
