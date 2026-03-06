use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::keys::service::KeyService;
use crate::sensors::service::SensorService;
use anyhow::Result;

#[tokio::test]
async fn test_add_nonce() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let sensor_service = SensorService::new(connection.clone());
    let service = NonceService::new(connection.clone());
    let key_service = KeyService::new(connection);

    let mac = "AA:BB:26:06:19:91";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = [26; 12];
    let key = [26; 32];
    let _ = key_service
        .add(key, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = service
        .add(nonce, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let created_nonce = sqlx::query_as::<_, Nonce>(
        r#"
        SELECT * FROM nonces WHERE id = ?
        "#,
    )
    .bind(nonce.id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created historical key");
    assert!(nonce.id > 0);
    assert_eq!(created_nonce, nonce);
    Ok(())
}
