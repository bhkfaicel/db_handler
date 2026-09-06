use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::history::keys::{model::HistoricalKey, service::HistoricalKeyService};
use crate::keys::error::KeyError;
use crate::sensors::service::SensorService;

#[tokio::test]
async fn test_add_key() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let repository = KeyService::new(connection.clone());
    let mac = "AA:BB:26:06:19:91";
    let sensor_service = SensorService::new(connection.clone());
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let key = [26; 32];
    let key = repository.add(key, mac).await?;
    let created_key = sqlx::query_as::<_, Key>(
        r#"
        SELECT * FROM keys WHERE id = ?
        "#,
    )
    .bind(key.id)
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created key");
    let created_historical_key = sqlx::query_as::<_, HistoricalKey>(
        r#"
    SELECT * FROM historical_keys WHERE value = ?
    "#,
    )
    .bind(key.value.clone())
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created historical key");
    assert!(key.id > 0);
    assert!(created_historical_key.id > 0);
    assert_eq!(created_key, key);
    Ok(())
}

#[tokio::test]
async fn test_add_key_that_exists_on_historical_key() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let key = [26; 32];
    let mac = "AA:BB:26:06:19:91";
    let historical_key_service = HistoricalKeyService::new(connection.clone());
    let _ = historical_key_service.add(key).await;
    let _ = sqlx::query_as::<_, HistoricalKey>(
        r#"
    SELECT * FROM historical_keys WHERE value = ?
    "#,
    )
    .bind(key.to_vec().clone())
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created historical key");

    let repository = KeyService::new(connection.clone());
    let result = repository.add(key, mac).await;
    assert!(
        result.is_err(),
        "Expected an error when adding a key that already used"
    );
    let find_key_result = sqlx::query_as::<_, Key>(
        r#"
        SELECT * FROM keys WHERE mac = ?
        "#,
    )
    .bind(mac)
    .fetch_one(&*connection.primary_db_client())
    .await;
    let err = find_key_result.expect_err("Expected RowNotFound error for nonexistent MAC");
    match err {
        sqlx::Error::RowNotFound => {}
        _ => panic!("Expected RowNotFound, got {:?}", err),
    }
    Ok(())
}

//TODO test service.get

#[tokio::test]
async fn test_get_by_mac_returns_key_when_exists() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());

    let mac = "AA:BB:26:06:19:91";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let key_value = [42; 32];
    let created_key = service.add(key_value, mac).await?;

    let fetched_key = service.get_by_mac(mac).await?;
    assert_eq!(fetched_key, created_key);
    Ok(())
}

#[tokio::test]
async fn test_get_by_mac_returns_error_when_absent() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());

    let mac = "AA:BB:26:06:19:92";
    let result = service.get_by_mac(mac).await;
    assert!(result.is_err(), "Expected error for mac without key");

    // Verify it's the correct error type
    match result {
        Err(KeyError::NotFound(msg)) => {
            assert!(
                msg.contains("AA:BB:26:06:19:92"),
                "Error should contain the MAC address"
            );
        }
        _ => panic!("Expected KeyError::NotFound"),
    }
    Ok(())
}

#[tokio::test]
async fn test_get_all_returns_valid_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());

    // Create several sensors and valid keys
    let macs = [
        "AA:BB:26:06:19:A1",
        "AA:BB:26:06:19:A2",
        "AA:BB:26:06:19:A3",
    ];

    let mut created_keys = Vec::new();
    for (i, mac) in macs.iter().enumerate() {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let key_value = [i as u8 + 50; 32];
        let key = service.add(key_value, mac).await?;
        created_keys.push(key);
    }

    let result = service.get_all().await?;
    assert_eq!(result.len(), 3, "Should return 3 valid keys");

    // Check that all the created keys are present
    for created_key in &created_keys {
        assert!(
            result
                .iter()
                .any(|k| k.id == created_key.id && k.mac == created_key.mac),
            "Expected key with id {} and mac {} not found",
            created_key.id,
            created_key.mac
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_get_all_returns_empty_when_no_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());

    let result = service.get_all().await?;
    assert_eq!(
        result.len(),
        0,
        "Should return empty vector when no keys exist"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_all_excludes_expired_keys() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());
    let historical_key_service = HistoricalKeyService::new(connection.clone());

    // Create a valid key through the service
    let valid_mac = "AA:BB:26:06:19:A4";
    let _ = sensor_service
        .add(valid_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let valid_key_value = [60; 32];
    let created_valid_key = service.add(valid_key_value, valid_mac).await?;

    // Create an expired key directly in the database with sqlx
    let expired_mac = "AA:BB:26:06:19:A5";
    let _ = sensor_service
        .add(expired_mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let expired_key_value = [70; 32];
    let _ = historical_key_service.add(expired_key_value).await?;
    use chrono::{DateTime, Duration, Utc};
    let mut past: DateTime<Utc> = Utc::now();
    past -= Duration::hours(1);
    sqlx::query("INSERT INTO keys (value, expire_at, mac) VALUES (?, ?, ?)")
        .bind(expired_key_value.to_vec())
        .bind(past)
        .bind(expired_mac)
        .execute(&*connection.primary_db_client())
        .await?;

    let result = service.get_all().await?;
    assert_eq!(result.len(), 1, "Should return only 1 valid key");
    assert_eq!(
        result[0].id, created_valid_key.id,
        "Should return the valid key"
    );
    assert_eq!(
        result[0].mac, created_valid_key.mac,
        "Should return the valid key's MAC"
    );
    Ok(())
}

/// Adding a key for a MAC that never had one must succeed: the internal
/// DELETE matches zero rows, which is `Ok`, so no branch may warn about or
/// swallow a failure that cannot happen for an absent previous key.
#[tokio::test]
async fn test_add_key_succeeds_when_no_previous_key_exists() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());

    let mac = "AA:BB:26:06:19:C9";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // No previous key exists for this MAC: the DELETE inside `add` deletes
    // zero rows and reports `Ok`, so the whole transaction must commit.
    let key = service.add([99; 32], mac).await?;
    assert!(key.id > 0);
    Ok(())
}

/// Rotation doubles as the retention trigger: the same transaction that
/// records the rotated-out key purges historical keys past their 30-day
/// window, while unexpired rows (the seeded valid one and the fresh
/// rotation row) must stay.
#[tokio::test]
async fn test_add_key_purges_expired_historical_keys() -> anyhow::Result<()> {
    use chrono::{Duration, Utc};

    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());
    let historical_key_service = HistoricalKeyService::new(connection.clone());

    let mac = "AA:BB:26:06:19:B1";
    let _ = sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Seed one expired historical key (raw SQL: the service always stamps
    // rows with a future expiration) and one still-valid row.
    let expired_value = [7; 32];
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)")
        .bind(expired_value.to_vec())
        .bind(Utc::now() - Duration::hours(1))
        .execute(&*connection.primary_db_client())
        .await?;
    let _ = historical_key_service.add([8; 32]).await?;

    // Rotation must purge the expired row and keep the valid ones.
    let _ = service.add([9; 32], mac).await?;

    let expired_left =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE value = ?")
            .bind(expired_value.to_vec())
            .fetch_optional(&*connection.primary_db_client())
            .await?;
    assert!(
        expired_left.is_none(),
        "the expired historical key must be purged by rotation"
    );

    let remaining =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE expire_at > ?")
            .bind(Utc::now())
            .fetch_all(&*connection.primary_db_client())
            .await?;
    assert_eq!(
        remaining.len(),
        2,
        "the seeded valid row and the fresh rotation row must stay"
    );
    Ok(())
}
