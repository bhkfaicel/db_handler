use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::keys::service::KeyService;
use crate::nonces::error::NonceError;
use crate::sensors::service::SensorService;
// Explicit anyhow Result: the service module re-exports the nonce domain
// `Result` alias through `use super::*`, which must not shadow the tests'
// return type.
use anyhow::{Ok, Result};
// Row trait needed for the `get("n")` column accessor on raw count queries.
use sqlx::Row;

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

/// The MAC format contract is enforced at the service's trust boundary:
/// a malformed address (line-protocol metacharacters) is refused as
/// `NonceError::InvalidMacFormat` BEFORE any database work, so nothing is
/// written to the anti-replay ledger.
#[tokio::test]
async fn test_add_nonce_rejects_invalid_mac() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let service = NonceService::new(connection);

    let malicious = "AA,BB CC=DD:EE:FF";
    let result = service.add([26; 12], malicious).await;
    match result {
        Err(NonceError::InvalidMacFormat(msg)) => {
            assert_eq!(msg, malicious, "the error must echo the offending MAC");
        }
        other => panic!("Expected NonceError::InvalidMacFormat, got {:?}", other),
    }
    // Nothing may have been written for the refused MAC.
    let stored: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces WHERE mac = ?")
        .bind(malicious)
        .fetch_one(&*pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .get("n");
    assert_eq!(stored, 0, "a refused MAC must leave the ledger untouched");
    Ok(())
}

/// Re-inserting the same nonce value must surface the dedicated typed
/// `NonceError::ReplayDetected` outcome: the service maps the database
/// `UNIQUE(value)` violation (constraint `nonces.value`) to the replay
/// variant instead of a generic database error. The refused insert leaves
/// the originally accepted row untouched, and a DIFFERENT value is still
/// accepted — only the seen value is burned.
#[tokio::test]
async fn duplicate_nonce_yields_replay_detected() -> Result<()> {
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
    let _ = key_service
        .add([26; 32], mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let value = [26u8; 12];
    let first = service
        .add(value, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(first.id > 0, "the first insert must be accepted");

    // Same value, same MAC: the database UNIQUE constraint is the arbiter,
    // so the duplicate is a typed replay, not a generic failure.
    let duplicate = service.add(value, mac).await;
    assert!(
        matches!(duplicate, Err(NonceError::ReplayDetected)),
        "expected ReplayDetected, got {:?}",
        duplicate
    );

    // The refused insert must not have touched the accepted row.
    let stored = sqlx::query_as::<_, Nonce>("SELECT * FROM nonces WHERE id = ?")
        .bind(first.id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert_eq!(stored, first, "the original ledger row must stay intact");

    // A different value for the same sensor is a fresh acceptance: the
    // ledger burns only the value it has actually seen.
    let other = service
        .add([27u8; 12], mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(
        other.id > 0 && other.id != first.id,
        "a different value must still be accepted"
    );
    Ok(())
}

/// The same nonce value presented by a DIFFERENT sensor must also be
/// reported as a replay: the final schema burns values GLOBALLY (a single
/// column-level `UNIQUE(value)` constraint; the composite value+mac index
/// was removed by the rebuild migration and the global scope is the
/// validated, strictly stronger anti-replay decision). An earlier draft of
/// this suite asserted the opposite (cross-sensor acceptance under the
/// per-MAC composite); that assertion described a schema tolerance that no
/// longer exists by decision.
#[tokio::test]
async fn same_nonce_on_different_mac_yields_replay_detected() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let sensor_service = SensorService::new(connection.clone());
    let service = NonceService::new(connection.clone());
    let key_service = KeyService::new(connection);

    // Two sensors with one key each: the second presentation needs a
    // valid foreign-key parent of its own.
    let mac_a = "AA:BB:26:06:19:91";
    let mac_b = "AA:BB:26:06:19:92";
    sensor_service
        .add(mac_a)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .add(mac_b)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    key_service
        .add([41; 32], mac_a)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    key_service
        .add([42; 32], mac_b)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let value = [26u8; 12];
    let first = service
        .add(value, mac_a)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(first.id > 0, "the first sensor's insert must be accepted");

    // Global burn: the same value from another sensor is a replay too.
    let replay = service.add(value, mac_b).await;
    assert!(
        matches!(replay, Err(NonceError::ReplayDetected)),
        "expected ReplayDetected for the cross-sensor duplicate, got {:?}",
        replay
    );

    // The refused insert stored nothing for the second sensor, and the
    // first sensor's accepted row is untouched.
    let remaining_b: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces WHERE mac = ?")
        .bind(mac_b)
        .fetch_one(&*pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .get("n");
    assert_eq!(remaining_b, 0, "no row may be stored for the refused mac");
    Ok(())
}

/// Burned nonces are retained only until the sensor's key row is deleted
/// at rotation: `KeyService::add` replaces the previous key row and the
/// `ON DELETE CASCADE` foreign key removes that sensor's nonces with it.
/// After the rotation the ledger no longer holds the value, so the same
/// nonce is accepted again — it belongs to the new key window. This is the
/// retention half of the contract: no explicit purge method exists, the
/// cascade is the only deletion path, and ledger growth is bounded by the
/// key-rotation window.
#[tokio::test]
async fn key_rotation_cascades_burned_nonces_away() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let sensor_service = SensorService::new(connection.clone());
    let service = NonceService::new(connection.clone());
    let key_service = KeyService::new(connection);

    let mac = "AA:BB:26:06:19:91";
    sensor_service
        .add(mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    key_service
        .add([26; 32], mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let value = [26u8; 12];
    let burned = service
        .add(value, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(
        burned.id > 0,
        "the nonce must be accepted under the first key"
    );

    // Rotation: re-adding a key for the same mac deletes the previous key
    // row, and the foreign-key cascade must take the sensor's nonces away.
    key_service
        .add([27; 32], mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let remaining: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces WHERE mac = ?")
        .bind(mac)
        .fetch_one(&*pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .get("n");
    assert_eq!(
        remaining, 0,
        "key rotation must cascade the sensor's nonces away"
    );

    // The value is burned no longer: the new key window starts clean, so
    // the very same nonce is accepted again.
    let reaccepted = service
        .add(value, mac)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert!(
        reaccepted.id > 0,
        "the same value must be acceptable again after its window's rotation"
    );
    Ok(())
}

/// Concurrent duplicate inserts: exactly one is accepted, the loser
/// observes the typed replay outcome. The arbiter is the database
/// UNIQUE constraint itself (DB-enforced), which is exactly the guarantee
/// a SELECT-then-INSERT probe could not provide.
#[tokio::test]
async fn concurrent_duplicate_inserts_yield_exactly_one_acceptance() -> Result<()> {
    use std::env;

    // File-backed database on purpose: the default in-memory test database
    // uses shared-cache table locks, under which a second concurrent
    // writer gets an immediate SQLITE_LOCKED instead of queueing, so the
    // exactly-once race is only deterministic on the WAL file branch that
    // production runs on (its busy timeout queues the loser behind the
    // winner's commit and the UNIQUE constraint then rejects it).
    let db_path = env::temp_dir().join(format!("dbh_nonce_race_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let config_path =
        env::temp_dir().join(format!("dbh_nonce_race_cfg_{}.json", std::process::id()));
    std::fs::write(
        &config_path,
        format!(
            r#"{{"primary_db_url":"sqlite://{}","history_db_url":"http://localhost:8086","history_db_org":"t","history_db_token":"t"}}"#,
            db_path.display()
        ),
    )?;

    // Environment variables are process-global: take the shared test guard
    // (it serializes access across parallel tests and restores the previous
    // value on drop).
    let guard = crate::test_env::ConfigFileGuard::acquire();
    guard.set(Some(config_path.to_str().unwrap()));

    let test_body = async {
        let connection = Connection::new().await?;
        let sensor_service = SensorService::new(connection.clone());
        let service = NonceService::new(connection.clone());
        let key_service = KeyService::new(connection);

        let mac = "AA:BB:26:06:19:91";
        sensor_service
            .add(mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        key_service
            .add([26; 32], mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Two writers submit the SAME value at the same time: exactly one
        // commit may win, and the loser must see the typed replay error.
        let value = [26u8; 12];
        let (first, second) = tokio::join!(service.add(value, mac), service.add(value, mac));

        let results = [first, second];
        let accepted = results.iter().filter(|r| r.is_ok()).count();
        let replayed = results
            .iter()
            .filter(|r| matches!(r, Err(NonceError::ReplayDetected)))
            .count();
        assert_eq!(
            accepted, 1,
            "exactly one concurrent insert may be accepted, got {:?}",
            results
        );
        assert_eq!(
            replayed, 1,
            "the losing insert must be reported as a replay, got {:?}",
            results
        );
        anyhow::Ok(())
    }
    .await;

    // Drop the guard: it restores the previous CONFIG_FILE value whatever
    // the test body's outcome.
    drop(guard);
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_file(&db_path);
    test_body
}
