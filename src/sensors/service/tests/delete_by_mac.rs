use crate::{
    db::connection::Connection, db::primary::service::Service, sensors::service::SensorService,
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_delete() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection.clone());
    let mac = "00:1B:44:11:3A:B9";
    service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    service
        .delete_by_mac(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let retrieved_sensor = service
        .get_by_mac(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(retrieved_sensor.is_none());
    Ok(())
}
