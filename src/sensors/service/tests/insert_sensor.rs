use crate::{
    db::connection::Connection, db::primary::service::Service, sensors::service::SensorService,
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_insert_sensor() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let id = service
        .add("00:1B:44:11:3A:B7")
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(id > 0);
    Ok(())
}
