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
async fn should_delete() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool);
    let mac = "00:1B:44:11:3A:B9";
    let sensor = Sensor::new(0, mac.to_string(), Status::OnUpdate);
    repository.insert(&sensor).await?;
    repository.delete_by_mac(mac).await?;
    let retrieved_sensor = repository.get_by_mac(mac).await?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
