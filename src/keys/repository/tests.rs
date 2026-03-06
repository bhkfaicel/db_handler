use chrono::{DateTime, Duration, Utc};

use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::history::keys::service::HistoricalKeyService as historicalKeyService;
use crate::nonces::service::NonceService;
use crate::sensors::service::SensorService;

async fn insert(pool: Arc<SqlitePool>, key: &Key) -> Result<u32, sqlx::Error> {
    let value = key.value.to_vec();
    let result = sqlx::query!(
        "INSERT INTO keys (value, expire_at,mac) VALUES (?, ?,?)",
        value,
        key.expire_at,
        key.mac
    )
    .execute(&*pool)
    .await?;
    Ok(result.last_insert_rowid() as u32)
}

#[tokio::test]
async fn test_get_by_mac() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());
    let nonce_service = NonceService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = [26; 32];
    let _ = historical_key_service.add(key.clone()).await?;
    let mut now: DateTime<Utc> = Utc::now();
    now = now + Duration::hours(1);
    let mut key_to_insert = Key {
        id: 0,
        value: key.clone().to_vec(),
        expire_at: now,
        mac: mac.clone(),
    };
    let id = insert(connection.primary_db_client().clone(), &key_to_insert).await?;
    key_to_insert.id = id;
    let _ = nonce_service.add([32; 12], &mac).await?;
    let _ = sqlx::query_as::<_, Key>(
        r#"
        SELECT * FROM keys WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created key");
    let result = repository.get_by_mac(&mac).await?;
    assert_eq!(result.is_some(), true);
    let result = result.unwrap();
    assert_eq!(result, key_to_insert);
    Ok(())
}

#[tokio::test]
async fn test_get_by_mac_no_key() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = [26; 32];
    let _ = historical_key_service.add(key.clone()).await?;
    let result = repository.get_by_mac(&mac).await?;
    assert_eq!(result.is_none(), true);
    Ok(())
}

#[tokio::test]
async fn test_get_by_mac_expired_returns_none() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());

    let mac = "AA:BB:26:06:19:93".to_string();
    let _ = sensor_service
        .add(&mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let key = [77; 32];
    let _ = historical_key_service.add(key.clone()).await?;
    let mut past: DateTime<Utc> = Utc::now();
    past = past - Duration::hours(1);
    let expired_key = Key {
        id: 0,
        value: key.clone().to_vec(),
        expire_at: past,
        mac: mac.clone(),
    };
    let id = insert(connection.primary_db_client().clone(), &expired_key).await?;
    let _ = sqlx::query_as::<_, Key>(
        r#"
        SELECT * FROM keys WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created expired key");
    let result = repository.get_by_mac(&mac).await?;
    assert!(result.is_none(), "Expected None for expired key");
    Ok(())
}

#[tokio::test]
async fn test_get_all_returns_valid_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());

    // Créer plusieurs capteurs et clés valides
    let macs = vec![
        "AA:BB:26:06:19:94".to_string(),
        "AA:BB:26:06:19:95".to_string(),
        "AA:BB:26:06:19:96".to_string(),
    ];

    let mut expected_keys = Vec::new();
    let mut now: DateTime<Utc> = Utc::now();
    now = now + Duration::hours(1);

    for (i, mac) in macs.iter().enumerate() {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let key_value = [i as u8; 32];
        let _ = historical_key_service.add(key_value.clone()).await?;
        let key_to_insert = Key {
            id: 0,
            value: key_value.to_vec(),
            expire_at: now,
            mac: mac.clone(),
        };
        let id = insert(connection.primary_db_client().clone(), &key_to_insert).await?;
        expected_keys.push(Key {
            id,
            value: key_value.to_vec(),
            expire_at: now,
            mac: mac.clone(),
        });
    }

    let result = repository.get_all().await?;
    assert_eq!(result.len(), 3, "Should return 3 valid keys");

    // Vérifier que toutes les clés attendues sont présentes
    for expected_key in &expected_keys {
        assert!(
            result
                .iter()
                .any(|k| k.id == expected_key.id && k.mac == expected_key.mac),
            "Expected key with id {} and mac {} not found",
            expected_key.id,
            expected_key.mac
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_get_all_excludes_expired_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());

    // Créer une clé valide
    let valid_mac = "AA:BB:26:06:19:97".to_string();
    let _ = sensor_service
        .add(&valid_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let valid_key_value = [10; 32];
    let _ = historical_key_service.add(valid_key_value.clone()).await?;
    let mut future: DateTime<Utc> = Utc::now();
    future = future + Duration::hours(1);
    let valid_key = Key {
        id: 0,
        value: valid_key_value.to_vec(),
        expire_at: future,
        mac: valid_mac.clone(),
    };
    let valid_id = insert(connection.primary_db_client().clone(), &valid_key).await?;

    // Créer une clé expirée
    let expired_mac = "AA:BB:26:06:19:98".to_string();
    let _ = sensor_service
        .add(&expired_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let expired_key_value = [20; 32];
    let _ = historical_key_service
        .add(expired_key_value.clone())
        .await?;
    let mut past: DateTime<Utc> = Utc::now();
    past = past - Duration::hours(1);
    let expired_key = Key {
        id: 0,
        value: expired_key_value.to_vec(),
        expire_at: past,
        mac: expired_mac.clone(),
    };
    let _expired_id = insert(connection.primary_db_client().clone(), &expired_key).await?;

    let result = repository.get_all().await?;
    assert_eq!(result.len(), 1, "Should return only 1 valid key");
    assert_eq!(result[0].id, valid_id, "Should return the valid key");
    assert_eq!(
        result[0].mac, valid_mac,
        "Should return the valid key's MAC"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_all_returns_empty_when_no_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let repository = KeyRepository::new(connection.primary_db_client().clone());

    let result = repository.get_all().await?;
    assert_eq!(
        result.len(),
        0,
        "Should return empty vector when no keys exist"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_all_returns_empty_when_all_expired() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let sensor_service = SensorService::new(connection.clone());
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    let historical_key_service = historicalKeyService::new(connection.clone());

    // Créer plusieurs clés expirées
    let macs = vec![
        "AA:BB:26:06:19:99".to_string(),
        "AA:BB:26:06:19:9A".to_string(),
    ];

    let mut past: DateTime<Utc> = Utc::now();
    past = past - Duration::hours(1);

    for (i, mac) in macs.iter().enumerate() {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let key_value = [i as u8; 32];
        let _ = historical_key_service.add(key_value.clone()).await?;
        let expired_key = Key {
            id: 0,
            value: key_value.to_vec(),
            expire_at: past,
            mac: mac.clone(),
        };
        let _ = insert(connection.primary_db_client().clone(), &expired_key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    let result = repository.get_all().await?;
    assert_eq!(
        result.len(),
        0,
        "Should return empty vector when all keys are expired"
    );
    Ok(())
}
