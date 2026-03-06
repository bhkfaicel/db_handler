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
    let sensor = Sensor::new(0, "00:1B:44:11:3A:B9".to_string(), Status::OnUpdate);
    let id = repository.insert(&sensor).await?;
    repository.delete_by_id(id).await?;
    let retrieved_sensor = repository.get_by_id(id).await?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
