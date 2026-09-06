use chrono::{DateTime, Duration, Utc};

use super::*;
use crate::db::connection::Connection;
use crate::db::primary::error::CommonError;
use crate::db::primary::service::Service;
use crate::history::keys::service::HistoricalKeyService as historicalKeyService;
use crate::nonces::service::NonceService;
use crate::sensors::service::SensorService;

// Test-local raw-SQL insert helper (does not go through the repository).
// `Result` is the repository module's one-generic alias re-exported by
// `use super::*`, so the full std path names the two-generic result here.
async fn insert(pool: Arc<SqlitePool>, key: &Key) -> std::result::Result<u32, sqlx::Error> {
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
    let _ = historical_key_service.add(key).await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
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
    assert!(result.is_some());
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
    let _ = historical_key_service.add(key).await?;
    let result = repository.get_by_mac(&mac).await?;
    assert!(result.is_none());
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
    let _ = historical_key_service.add(key).await?;
    let mut past: DateTime<Utc> = Utc::now();
    past -= Duration::hours(1);
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

    // Create several sensors and valid keys
    let macs = [
        "AA:BB:26:06:19:94".to_string(),
        "AA:BB:26:06:19:95".to_string(),
        "AA:BB:26:06:19:96".to_string(),
    ];

    let mut expected_keys = Vec::new();
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);

    for (i, mac) in macs.iter().enumerate() {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let key_value = [i as u8; 32];
        let _ = historical_key_service.add(key_value).await?;
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

    // Check that all the expected keys are present
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

    // Create a valid key
    let valid_mac = "AA:BB:26:06:19:97".to_string();
    let _ = sensor_service
        .add(&valid_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let valid_key_value = [10; 32];
    let _ = historical_key_service.add(valid_key_value).await?;
    let mut future: DateTime<Utc> = Utc::now();
    future += Duration::hours(1);
    let valid_key = Key {
        id: 0,
        value: valid_key_value.to_vec(),
        expire_at: future,
        mac: valid_mac.clone(),
    };
    let valid_id = insert(connection.primary_db_client().clone(), &valid_key).await?;

    // Create an expired key
    let expired_mac = "AA:BB:26:06:19:98".to_string();
    let _ = sensor_service
        .add(&expired_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let expired_key_value = [20; 32];
    let _ = historical_key_service.add(expired_key_value).await?;
    let mut past: DateTime<Utc> = Utc::now();
    past -= Duration::hours(1);
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

    // Create several expired keys
    let macs = [
        "AA:BB:26:06:19:99".to_string(),
        "AA:BB:26:06:19:9A".to_string(),
    ];

    let mut past: DateTime<Utc> = Utc::now();
    past -= Duration::hours(1);

    for (i, mac) in macs.iter().enumerate() {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let key_value = [i as u8; 32];
        let _ = historical_key_service.add(key_value).await?;
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

#[tokio::test]
async fn test_insert_duplicate_value_yields_unique_violation() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let repository = KeyRepository::new(connection.primary_db_client().clone());
    // Satisfy the `keys` foreign keys first (one sensor per MAC, one
    // historical key for the value), like every other keys fixture.
    let mac_first = "AA:BB:26:06:19:B1";
    let mac_second = "AA:BB:26:06:19:B2";
    let key_value = [91; 32];
    let sensor_service = SensorService::new(connection.clone());
    for mac in [mac_first, mac_second] {
        sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }
    // The file imports the type with the `historicalKeyService` alias; the
    // fixture reuses it to satisfy the `keys.value` foreign key.
    historicalKeyService::new(connection.clone())
        .add(key_value)
        .await?;
    // Only the value is duplicated between the two inserts (the MACs
    // differ), so the only constraint SQLite can report is UNIQUE(value):
    // the assertion on the constraint name stays deterministic.
    let first = Key {
        id: 0,
        value: key_value.to_vec(),
        expire_at: Utc::now() + Duration::hours(1),
        mac: mac_first.to_string(),
    };
    let duplicate = Key {
        id: 0,
        value: key_value.to_vec(),
        expire_at: Utc::now() + Duration::hours(1),
        mac: mac_second.to_string(),
    };

    // The first insert of the value succeeds inside the transaction.
    let mut tx = connection.primary_db_client().begin().await?;
    let _ = repository.insert_with_transaction(&mut tx, &first).await?;

    // The second insert of the same value violates the UNIQUE(value)
    // constraint and must surface as the structured unique-violation error
    // carrying the constraint name SQLite reports in its message.
    let result = repository
        .insert_with_transaction(&mut tx, &duplicate)
        .await;
    match result {
        Err(CommonError::UniqueViolation { constraint }) => {
            assert_eq!(constraint, "keys.value");
        }
        other => panic!("Expected UniqueViolation, got {:?}", other),
    }

    tx.rollback().await?;
    Ok(())
}
