use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::history::keys::{model::HistoricalKey, service::HistoricalKeyService};
use crate::keys::error::KeyError;
use crate::nonces::service::NonceService;
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

/// The MAC format contract is enforced at the service's trust boundary:
/// a malformed address (line-protocol metacharacters) is refused as
/// `KeyError::InvalidMacFormat` BEFORE any database work, so no rotation
/// transaction is started and no row can be touched.
#[tokio::test]
async fn test_add_key_rejects_invalid_mac() -> anyhow::Result<()> {
    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());

    let malicious = "AA,BB CC=DD:EE:FF";
    let result = service.add([26; 32], malicious).await;
    match result {
        Err(KeyError::InvalidMacFormat(msg)) => {
            assert_eq!(msg, malicious, "the error must echo the offending MAC");
        }
        other => panic!("Expected KeyError::InvalidMacFormat, got {:?}", other),
    }
    Ok(())
}

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

/// Foreign-key safety belt: an expired historical key still referenced by
/// a LIVE key row must survive the retention purge — deleting it would
/// violate `keys.value -> historical_keys.value` (NO ACTION) and abort the
/// rotation — while an expired, unreferenced historical row goes. The
/// rotation must succeed through both outcomes without any error.
#[tokio::test]
async fn test_add_key_purge_keeps_historical_key_referenced_by_live_key() -> anyhow::Result<()> {
    use chrono::{Duration, Utc};

    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());

    // One sensor keeps a live key, the other performs the rotation whose
    // transaction runs the retention purges.
    let live_mac = "AA:BB:26:06:19:C1";
    let rotating_mac = "AA:BB:26:06:19:C2";
    for mac in [live_mac, rotating_mac] {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    // Expired historical row still referenced by the LIVE key of
    // `live_mac` (synthetic state: the guard must hold regardless).
    let referenced_value = [7; 32];
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)")
        .bind(referenced_value.to_vec())
        .bind(Utc::now() - Duration::hours(1))
        .execute(&*connection.primary_db_client())
        .await?;
    sqlx::query("INSERT INTO keys (value, expire_at, mac) VALUES (?, ?, ?)")
        .bind(referenced_value.to_vec())
        .bind(Utc::now() + Duration::hours(1))
        .bind(live_mac)
        .execute(&*connection.primary_db_client())
        .await?;

    // Expired historical row referenced by no key at all.
    let orphan_value = [8; 32];
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)")
        .bind(orphan_value.to_vec())
        .bind(Utc::now() - Duration::hours(1))
        .execute(&*connection.primary_db_client())
        .await?;

    // The rotation must not error, must delete the unreferenced expired
    // row and must keep the referenced one.
    let _ = service.add([9; 32], rotating_mac).await?;

    let referenced_left =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE value = ?")
            .bind(referenced_value.to_vec())
            .fetch_optional(&*connection.primary_db_client())
            .await?;
    assert!(
        referenced_left.is_some(),
        "the historical key referenced by a live key must survive the purge"
    );
    let orphan_left =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE value = ?")
            .bind(orphan_value.to_vec())
            .fetch_optional(&*connection.primary_db_client())
            .await?;
    assert!(
        orphan_left.is_none(),
        "the expired unreferenced historical key must be purged"
    );
    // The referencing live key itself must be untouched by the purges.
    let live_key_left = sqlx::query("SELECT 1 FROM keys WHERE mac = ?")
        .bind(live_mac)
        .fetch_optional(&*connection.primary_db_client())
        .await?;
    assert!(
        live_key_left.is_some(),
        "the live key must not be affected by the retention purges"
    );
    Ok(())
}

/// Regression: a sensor offline for more than the 30-day historical
/// retention window leaves an expired key row whose historical row is
/// expired too. The unguarded historical purge used to delete that
/// referenced row, violating `keys.value -> historical_keys.value` and
/// aborting EVERY subsequent rotation (key-rotation denial of service).
/// The rotation of another sensor must now succeed: the expired key row is
/// purged first, its anti-replay nonces cascade away
/// (`nonces.mac -> keys.mac` ON DELETE CASCADE), the orphaned historical
/// row becomes unreferenced and is purged, and the rotating sensor gets
/// its new key and historical row.
#[tokio::test]
async fn test_add_key_rotation_succeeds_with_offline_sensor_orphan() -> anyhow::Result<()> {
    use chrono::{Duration, Utc};

    let connection = Connection::new().await?;
    let service = KeyService::new(connection.clone());
    let sensor_service = SensorService::new(connection.clone());
    let nonce_service = NonceService::new(connection.clone());

    // The long-offline sensor: its key expired 31 days ago and its
    // historical row (30-day retention) expired as well. One nonce is
    // attached to that key row to pin the cascade.
    let offline_mac = "AA:BB:26:06:19:D1";
    let rotating_mac = "AA:BB:26:06:19:D2";
    for mac in [offline_mac, rotating_mac] {
        let _ = sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }
    let orphan_value = [31; 32];
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, ?)")
        .bind(orphan_value.to_vec())
        .bind(Utc::now() - Duration::days(31))
        .execute(&*connection.primary_db_client())
        .await?;
    sqlx::query("INSERT INTO keys (value, expire_at, mac) VALUES (?, ?, ?)")
        .bind(orphan_value.to_vec())
        .bind(Utc::now() - Duration::days(31))
        .bind(offline_mac)
        .execute(&*connection.primary_db_client())
        .await?;
    let _ = nonce_service.add([55; 12], offline_mac).await?;

    // Another sensor rotates: before the dependency-safe ordering this
    // aborted on the foreign-key violation.
    let new_key = service.add([41; 32], rotating_mac).await?;

    // The expired orphan key row is gone and its nonce cascaded with it.
    let orphan_key_left = sqlx::query("SELECT 1 FROM keys WHERE mac = ?")
        .bind(offline_mac)
        .fetch_optional(&*connection.primary_db_client())
        .await?;
    assert!(
        orphan_key_left.is_none(),
        "the offline sensor's expired key row must be purged by the rotation"
    );
    let nonce_left = sqlx::query("SELECT 1 FROM nonces WHERE mac = ?")
        .bind(offline_mac)
        .fetch_optional(&*connection.primary_db_client())
        .await?;
    assert!(
        nonce_left.is_none(),
        "the offline sensor's nonces must cascade away with its key row"
    );
    // The orphaned historical row became unreferenced and was purged.
    let orphan_historical =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE value = ?")
            .bind(orphan_value.to_vec())
            .fetch_optional(&*connection.primary_db_client())
            .await?;
    assert!(
        orphan_historical.is_none(),
        "the orphaned expired historical key must be purged once unreferenced"
    );
    // The rotating sensor holds its new key and the rotation's historical
    // row exists.
    assert!(new_key.id > 0);
    let new_historical =
        sqlx::query_as::<_, HistoricalKey>("SELECT * FROM historical_keys WHERE value = ?")
            .bind(new_key.value.clone())
            .fetch_optional(&*connection.primary_db_client())
            .await?;
    assert!(
        new_historical.is_some(),
        "the rotation's historical key row must exist"
    );
    Ok(())
}
