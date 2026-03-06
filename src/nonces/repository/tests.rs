use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::keys::service::KeyService;
use crate::sensors::service::SensorService;
use anyhow::Result;

#[tokio::test]
async fn test_insert_nonce() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = NonceRepository::new(pool.clone());
    let key_service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = vec![26; 32];
    let _ = key_service
        .add([26; 32], &mac.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let mut nonce = Nonce {
        id: 0,
        value: key.clone(),
        mac,
    };
    let id = repository
        .insert(&nonce)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let created_nonce = sqlx::query_as::<_, Nonce>(
        r#"
        SELECT * FROM nonces WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created nonce");
    assert!(id > 0);
    nonce.id = id;
    assert_eq!(nonce, created_nonce);
    Ok(())
}

#[tokio::test]
async fn test_insert_nonce_mac_does_not_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let repository = NonceRepository::new(connection.primary_db_client().clone());
    let sensor_service = SensorService::new(connection.clone());
    let key_service = KeyService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = vec![26; 32];
    let _ = key_service
        .add([26; 32], &mac.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = Nonce {
        id: 0,
        value: key.clone(),
        mac: mac + "1",
    };
    let insert_result = repository.insert(&nonce).await;
    let err = insert_result.expect_err("Expected FOREIGN KEY constraint failed");
    match err {
        sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => Ok(()),
        other => {
            panic!("Expected FOREIGN KEY violation, got {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_insert_nonce_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = NonceRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let key_service = KeyService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = vec![26; 32];
    let _ = key_service
        .add([26; 32], &mac.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let nonce = Nonce {
        id: 0,
        value: key.clone(),
        mac: mac,
    };
    let _ = repository
        .insert(&nonce)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let insert_result = repository.insert(&nonce).await;
    let err = insert_result.expect_err("UNIQUE constraint failed");
    match err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => Ok(()),
        other => {
            panic!("UNIQUE constraint failed, got {:?}", other);
        }
    }
}
