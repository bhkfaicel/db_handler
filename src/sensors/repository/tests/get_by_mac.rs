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
async fn should_return_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool);
    let mac = "00:1B:44:11:3A:B8";
    let sensor = Sensor::new(0, mac.to_string(), Status::InProgress);
    let id = repository.insert(&sensor).await?;
    let retrieved_sensor = repository.get_by_mac(mac).await?;
    assert!(retrieved_sensor.is_some());
    let retrieved_sensor = retrieved_sensor.unwrap();
    assert_eq!(retrieved_sensor.mac, mac);
    assert_eq!(retrieved_sensor.status, Status::InProgress);
    assert_eq!(retrieved_sensor.id, id);
    Ok(())
}
