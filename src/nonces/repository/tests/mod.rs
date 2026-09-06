use super::*;
use crate::db::connection::Connection;
use crate::db::primary::error::CommonError;
use crate::db::primary::service::Service;
use crate::keys::service::KeyService;
use crate::sensors::service::SensorService;
// Explicit anyhow Result: the repository module now re-exports the
// `CommonError`-based `Result` alias, which must not shadow the tests'
// return type.
use anyhow::{Ok, Result};
use sqlx::Row;

/// Name of the migration file that rebuilds `nonces` without the composite
/// unique index: it is both the schema-migrations ledger key and the file
/// whose embedded SQL the tests replay.
const REBUILD_MIGRATION_NAME: &str = "20260906000002_rebuild_nonces_drop_composite_unique.sql";

/// Content of the rebuild migration, embedded so the tests exercise the
/// exact shipped SQL (the production runner reads the same embedded files).
const REBUILD_MIGRATION_SQL: &str =
    include_str!("../../../../migrations/20260906000002_rebuild_nonces_drop_composite_unique.sql");

/// The pre-rebuild `nonces` schema, verbatim from the legacy init migration:
/// existing deployments carry the composite `UNIQUE (value, mac)` index that
/// the rebuild migration removes.
const LEGACY_NONCES_DDL: &str = "CREATE TABLE IF NOT EXISTS nonces (\
     id INTEGER PRIMARY KEY AUTOINCREMENT,\
     value BLOB NOT NULL UNIQUE,\
     mac TEXT NOT NULL,\
     UNIQUE (value, mac),\
     FOREIGN KEY (mac) REFERENCES keys(mac) ON DELETE CASCADE)";

/// Number of foreign-key violations reported by `PRAGMA foreign_key_check`
/// over the whole database: zero is the rebuild's post-migration invariant.
async fn foreign_key_check_violations(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM pragma_foreign_key_check")
        .fetch_one(pool)
        .await
        .expect("foreign_key_check must be queryable")
        .get("n")
}

/// True while the composite unique auto-index (`sqlite_autoindex_nonces_2`)
/// still exists on `nonces`: present on the legacy schema, gone after a
/// successful rebuild.
async fn composite_unique_index_present(pool: &sqlx::SqlitePool) -> bool {
    sqlx::query(
        "SELECT COUNT(*) AS n FROM pragma_index_list('nonces') \
         WHERE name = 'sqlite_autoindex_nonces_2'",
    )
    .fetch_one(pool)
    .await
    .expect("index_list must be queryable")
    .get::<i64, _>("n")
        > 0
}

/// Seeds the parent rows one nonce needs (sensor, historical key, key) with
/// raw SQL so the migration tests stay decoupled from the domain services.
async fn seed_nonce_parents(pool: &sqlx::SqlitePool, key_value: &[u8], mac: &str) {
    sqlx::query("INSERT INTO sensors (mac, status) VALUES (?, 'Installed')")
        .bind(mac)
        .execute(pool)
        .await
        .expect("sensor insert");
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, datetime('now'))")
        .bind(key_value)
        .execute(pool)
        .await
        .expect("historical key insert");
    sqlx::query("INSERT INTO keys (value, expire_at, mac) VALUES (?, datetime('now'), ?)")
        .bind(key_value)
        .bind(mac)
        .execute(pool)
        .await
        .expect("key insert");
}

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
        // The repository reports failures as `CommonError`; the raw sqlx
        // foreign-key error is preserved inside the generic `Database`
        // variant, so the original violation kind stays assertable.
        CommonError::Database(sqlx::Error::Database(db_err))
            if db_err.is_foreign_key_violation() =>
        {
            Ok(())
        }
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
        mac,
    };
    let _ = repository
        .insert(&nonce)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let insert_result = repository.insert(&nonce).await;
    let err = insert_result.expect_err("UNIQUE constraint failed");
    match err {
        // A fully duplicated nonce violates the inline UNIQUE(value); the
        // composite UNIQUE(value, mac) that used to fire alongside it was
        // removed by the table-rebuild migration, so SQLite now reports
        // `nonces.value`. The assertion accepts both constraint names
        // (typed, not message-based) to cover databases in either schema
        // state.
        CommonError::UniqueViolation { constraint }
            if constraint == "nonces.value" || constraint == "nonces.value, nonces.mac" =>
        {
            Ok(())
        }
        other => {
            panic!(
                "Expected UniqueViolation on a nonces constraint, got {:?}",
                other
            );
        }
    }
}

/// Fresh-database path after the rebuild migration: the column-level UNIQUE
/// still rejects duplicates (same value on the same MAC and the same value
/// on a different MAC), the composite unique index is gone, deleting the
/// parent key still cascades to the nonce, and the migrated schema reports
/// zero foreign-key violations.
#[tokio::test]
async fn test_rebuilt_nonces_keep_uniques_cascade_and_fk_integrity() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = NonceRepository::new(pool.clone());
    let sensor_service = SensorService::new(connection.clone());
    let key_service = KeyService::new(connection.clone());

    // Two sensors with one key each: the cross-MAC case needs a second
    // valid parent, and key values are UNIQUE so the two keys differ.
    let mac_a = "AA:BB:26:06:19:91".to_string();
    let mac_b = "AA:BB:26:06:19:92".to_string();
    sensor_service
        .add(&mac_a)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    sensor_service
        .add(&mac_b)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    key_service
        .add([41; 32], &mac_a)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    key_service
        .add([42; 32], &mac_b)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let nonce = Nonce {
        id: 0,
        value: vec![26; 32],
        mac: mac_a.clone(),
    };
    repository
        .insert(&nonce)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // The same value on the same MAC is still rejected, and the reported
    // constraint is deterministically the column-level one: the composite
    // index no longer exists to fire instead.
    let duplicate = repository.insert(&nonce).await;
    match duplicate.expect_err("a repeated nonce value must be rejected") {
        CommonError::UniqueViolation { constraint } if constraint == "nonces.value" => {}
        other => panic!(
            "Expected UniqueViolation on nonces.value after the rebuild, got {:?}",
            other
        ),
    }

    // The same value on a DIFFERENT MAC is rejected too: the column-level
    // UNIQUE is stricter than the removed composite, which would have
    // accepted this row.
    let cross_mac = repository
        .insert(&Nonce {
            id: 0,
            value: vec![26; 32],
            mac: mac_b.clone(),
        })
        .await;
    match cross_mac.expect_err("the same value on a different MAC must be rejected") {
        CommonError::UniqueViolation { constraint } if constraint == "nonces.value" => {}
        other => panic!(
            "Expected UniqueViolation on nonces.value for the cross-MAC duplicate, got {:?}",
            other
        ),
    }

    // The rebuild actually removed the composite index.
    assert!(
        !composite_unique_index_present(&pool).await,
        "the composite unique index must be gone on a freshly migrated database"
    );
    assert_eq!(
        foreign_key_check_violations(&pool).await,
        0,
        "the migrated database must report zero foreign-key violations"
    );

    // Deleting the parent key still cascades to the nonce.
    sqlx::query("DELETE FROM keys WHERE mac = ?")
        .bind(&mac_a)
        .execute(&*pool)
        .await?;
    let remaining: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces WHERE mac = ?")
        .bind(&mac_a)
        .fetch_one(&*pool)
        .await?
        .get("n");
    assert_eq!(
        remaining, 0,
        "deleting the parent key must still cascade to nonces"
    );
    assert_eq!(
        foreign_key_check_violations(&pool).await,
        0,
        "no foreign-key violation may appear after the cascade"
    );
    Ok(())
}

/// Legacy-deployment path: a database carrying the old composite-unique
/// schema plus sample data and a pre-filled migration ledger (every file
/// applied except the rebuild) is migrated by a plain production boot; the
/// runner then applies ONLY the rebuild migration, and data, constraints,
/// cascade and the ledger row all come out correct.
#[tokio::test]
async fn test_legacy_deployment_rebuild_preserves_data_and_ledger() -> Result<()> {
    use std::env;

    // Isolated file-backed database for this test only (the boot-migration
    // test in the core module uses the same pattern).
    let db_path = env::temp_dir().join(format!("dbh_nonces_legacy_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let config_path = env::temp_dir().join(format!("dbh_nonces_cfg_{}.json", std::process::id()));
    std::fs::write(
        &config_path,
        format!(
            r#"{{"primary_db_url":"sqlite://{}","history_db_url":"http://localhost:8086","history_db_org":"t","history_db_token":"t"}}"#,
            db_path.display()
        ),
    )?;

    // Environment variables are process-global: save and restore so this
    // test cannot poison later ones (single-threaded suite).
    let original = env::var("CONFIG_FILE").ok();
    env::set_var("CONFIG_FILE", &config_path);

    let test_body = async {
        // Boot 1 on a fresh file: every embedded migration applies. This is
        // an existing deployment except for nonces, which is reset to the
        // exact legacy schema below.
        let connection = Connection::new().await?;
        let pool = connection.primary_db_client().clone();

        // Rebuild the legacy deployment state: old nonces schema, sample
        // data, ledger containing every applied file except the rebuild
        // migration (exactly what a deployment upgraded before this
        // migration ships looks like).
        sqlx::query("DROP TABLE nonces").execute(&*pool).await?;
        sqlx::query(LEGACY_NONCES_DDL).execute(&*pool).await?;
        seed_nonce_parents(&pool, &[7, 8], "AA:BB:26:06:19:93").await;
        sqlx::query("INSERT INTO nonces (id, value, mac) VALUES (11, ?, 'AA:BB:26:06:19:93')")
            .bind([9u8, 10].as_slice())
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM schema_migrations WHERE name = ?")
            .bind(REBUILD_MIGRATION_NAME)
            .execute(&*pool)
            .await?;
        let ledger_before: i64 = sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations")
            .fetch_one(&*pool)
            .await?
            .get("n");
        assert!(
            composite_unique_index_present(&pool).await,
            "the legacy schema must carry the composite unique index before the rebuild"
        );

        drop(pool);
        drop(connection);

        // Boot 2: the production runner skips every ledgered file and
        // therefore applies ONLY the rebuild migration.
        let connection = Connection::new().await?;
        let pool = connection.primary_db_client().clone();

        // Data preserved: the seeded id, value and mac come out of the
        // rebuilt table unchanged.
        let row = sqlx::query("SELECT id, value, mac FROM nonces")
            .fetch_one(&*pool)
            .await?;
        let id: i64 = row.get("id");
        let value: Vec<u8> = row.get("value");
        let mac: String = row.get("mac");
        assert_eq!(id, 11, "row identifiers must survive the rebuild");
        assert_eq!(
            value,
            vec![9u8, 10],
            "nonce values must survive the rebuild"
        );
        assert_eq!(
            mac, "AA:BB:26:06:19:93",
            "nonce MACs must survive the rebuild"
        );

        // Constraints enforced: composite gone, column-level UNIQUE kept.
        assert!(
            !composite_unique_index_present(&pool).await,
            "the composite unique index must be gone after the rebuild"
        );
        let duplicate =
            sqlx::query("INSERT INTO nonces (value, mac) VALUES (?, 'AA:BB:26:06:19:93')")
                .bind([9u8, 10].as_slice())
                .execute(&*pool)
                .await;
        assert!(
            duplicate.is_err(),
            "a repeated nonce value must still be rejected after the rebuild"
        );

        // Foreign keys intact: zero violations and the cascade still works.
        assert_eq!(
            foreign_key_check_violations(&pool).await,
            0,
            "the migrated database must report zero foreign-key violations"
        );
        sqlx::query("DELETE FROM keys WHERE mac = 'AA:BB:26:06:19:93'")
            .execute(&*pool)
            .await?;
        let remaining: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces")
            .fetch_one(&*pool)
            .await?
            .get("n");
        assert_eq!(
            remaining, 0,
            "deleting the parent key must still cascade to nonces after the rebuild"
        );

        // Ledger: exactly one new row, the rebuild migration, recorded by
        // the same transaction that applied it.
        let ledger_after: i64 = sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations")
            .fetch_one(&*pool)
            .await?
            .get("n");
        assert_eq!(
            ledger_after,
            ledger_before + 1,
            "exactly one migration may be recorded by the second boot"
        );
        let recorded: i64 =
            sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations WHERE name = ?")
                .bind(REBUILD_MIGRATION_NAME)
                .fetch_one(&*pool)
                .await?
                .get("n");
        assert_eq!(
            recorded, 1,
            "the rebuild migration must be recorded exactly once"
        );

        drop(pool);
        drop(connection);
        Ok(())
    }
    .await;

    match original {
        Some(v) => env::set_var("CONFIG_FILE", v),
        None => {
            env::remove_var("CONFIG_FILE");
        }
    }
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_file(&db_path);
    test_body
}

/// Interruption/rollback path: a copy of the rebuild migration with an
/// injected failing final statement runs through the runner's ledger
/// transaction pattern; the failure must roll the whole transaction back —
/// original table with its composite index intact, no nonces_new residue,
/// no ledger row.
#[tokio::test]
async fn test_interrupted_rebuild_rolls_back_completely() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();

    // Restore the legacy deployment state on this fresh database: old
    // nonces schema with data, rebuild migration not yet ledgered.
    sqlx::query("DROP TABLE nonces").execute(&*pool).await?;
    sqlx::query(LEGACY_NONCES_DDL).execute(&*pool).await?;
    seed_nonce_parents(&pool, &[3, 4], "00:1B:44:11:3A:E2").await;
    sqlx::query("INSERT INTO nonces (id, value, mac) VALUES (5, ?, '00:1B:44:11:3A:E2')")
        .bind([1u8, 2].as_slice())
        .execute(&*pool)
        .await?;
    sqlx::query("DELETE FROM schema_migrations WHERE name = ?")
        .bind(REBUILD_MIGRATION_NAME)
        .execute(&*pool)
        .await?;

    // Copy of the migration with an injected failing final statement: the
    // insert targets a MAC absent from keys, so the foreign key fails after
    // the DDL statements already succeeded inside the transaction.
    let interrupted = format!(
        "{REBUILD_MIGRATION_SQL}\n\
         INSERT INTO nonces (value, mac) VALUES (X'00', 'no-such-key-mac');"
    );

    // Exact runner transaction pattern (begin, execute the migration file,
    // commit): an interrupted boot abandons the transaction and SQLite
    // rolls it back — reproduced here with an explicit awaited rollback so
    // the assertions cannot race the rollback.
    let mut tx = pool.begin().await?;
    let migration_result = sqlx::query(&interrupted).execute(&mut *tx).await;
    assert!(
        migration_result.is_err(),
        "the migration copy with the injected failing final statement must fail"
    );
    tx.rollback().await?;

    // Full rollback: legacy table and data intact, composite index still
    // present, no nonces_new residue, no ledger row.
    assert!(
        composite_unique_index_present(&pool).await,
        "the rollback must restore the original table with its composite index"
    );
    let nonces: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nonces")
        .fetch_one(&*pool)
        .await?
        .get("n");
    assert_eq!(
        nonces, 1,
        "the original nonce row must survive the rollback"
    );
    let residue: i64 = sqlx::query(
        "SELECT COUNT(*) AS n FROM sqlite_master \
         WHERE type = 'table' AND name = 'nonces_new'",
    )
    .fetch_one(&*pool)
    .await?
    .get("n");
    assert_eq!(residue, 0, "no nonces_new residue may survive the rollback");
    assert_eq!(
        foreign_key_check_violations(&pool).await,
        0,
        "the rolled-back database must report zero foreign-key violations"
    );
    let ledger: i64 = sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations WHERE name = ?")
        .bind(REBUILD_MIGRATION_NAME)
        .fetch_one(&*pool)
        .await?
        .get("n");
    assert_eq!(ledger, 0, "no ledger row may survive the rollback");
    Ok(())
}
