use super::*;
use crate::sensors::service::SensorService;
use chrono::DateTime;
use std::io::Write;
use std::time::Duration;

mod support;

async fn setup(
    status: u16,
) -> (
    BatteryService,
    std::thread::JoinHandle<String>,
    Arc<SqlitePool>,
) {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20250226015426_init.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO sensors (id, mac, status) VALUES (1, 'AA:BB:CC:DD:EE:FF', 'active')")
        .execute(&pool)
        .await
        .unwrap();
    // Apply to an existing database containing a sensor.
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20260921000000_sensor_battery.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let (addr, server) = support::history_fake(status);
    let pool = Arc::new(pool);
    let service = BatteryService::with_parts(
        pool.clone(),
        Arc::new(HistoryDbClient::new(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            format!("http://{addr}").parse().unwrap(),
            "test".into(),
            "test".into(),
        )),
        "battery".into(),
    );
    (service, server, pool)
}

fn reading() -> BatteryReading {
    BatteryReading {
        sensor_id: 1,
        resting_mv: 4100,
        loaded_mv: 4050,
        received_at: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    }
}

#[tokio::test]
async fn battery_migration_latest_history_and_cascade() {
    let (service, server, pool) = setup(204).await;
    assert!(service.latest(1).await.unwrap().is_none());
    let sample = reading();
    service.record(&sample).await.unwrap();
    assert_eq!(service.latest(1).await.unwrap(), Some(sample));
    assert_eq!(
        server.join().unwrap(),
        "sensor_battery,sensor_id=1 resting_mv=4100i,loaded_mv=4050i 1800000000000000000"
    );
    sqlx::query("DELETE FROM sensors WHERE id=1")
        .execute(pool.as_ref())
        .await
        .unwrap();
    assert!(service.latest(1).await.unwrap().is_none());
}

#[tokio::test]
async fn battery_influx_failure_keeps_sqlite_and_updates_latest() {
    let (service, server, _pool) = setup(500).await;
    let mut sample = reading();
    assert!(service.record(&sample).await.is_err());
    server.join().unwrap();
    assert_eq!(service.latest(1).await.unwrap(), Some(sample.clone()));
    // Closed endpoint: a newer SQLite value still replaces the previous one.
    sample.received_at += chrono::Duration::seconds(1);
    sample.resting_mv = 4000;
    assert!(service.record(&sample).await.is_err());
    assert_eq!(service.latest(1).await.unwrap(), Some(sample.clone()));
    assert!(service.record(&reading()).await.is_err());
    assert_eq!(service.latest(1).await.unwrap(), Some(sample));
}

#[tokio::test]
async fn battery_unknown_sensor_cannot_create_orphan() {
    let (service, server, _pool) = setup(204).await;
    let mut sample = reading();
    sample.sensor_id = 999;
    assert!(service.record(&sample).await.is_err());
    assert!(service.latest(999).await.unwrap().is_none());
    service.record(&reading()).await.unwrap();
    server.join().unwrap();
}

/// Production-wiring proof through the real `Connection::new()` constructor:
/// the migration runner creates the database (including `sensor_battery`),
/// the parent sensor is created through the regular `SensorService`, and the
/// service is built with the `Service` trait, exactly as production callers
/// do. A local fake history endpoint answering HTTP 500 makes the write
/// failure deterministic (never the environment-dependent `influxdb:8086`):
/// `record` must surface it as `BatteryError::HistoryWrite` while the SQLite
/// latest value stays committed and readable through `latest`.
#[tokio::test]
async fn battery_production_wiring_record_history_write_failure_keeps_latest() {
    // Local one-shot fake answering HTTP 500: the failure is deterministic.
    let (addr, server) = support::history_fake(500);
    // Temporary configuration: a fresh `:memory:` primary database for the
    // migration runner and the history client pointed at the local fake.
    let config_path = std::env::temp_dir().join(format!(
        "dbh_battery_wiring_cfg_{}.json",
        std::process::id()
    ));
    // Pre-clean a stale file left by a crashed prior run, then create the
    // file exclusively: exclusive creation (`O_CREAT|O_EXCL`) never follows
    // a pre-planted symlink, so nothing pre-existing at that path can be
    // clobbered — a concurrent name collision just fails this open
    // (`AlreadyExists`) and errors the test.
    // ponytail: residual is a collision-induced flake, never a clobber;
    // move to a private temp dir only if the test ever carries real secrets
    // or guaranteed run-to-completion becomes a requirement.
    let _ = std::fs::remove_file(&config_path);
    let mut config_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_path)
        .unwrap();
    config_file
        .write_all(
            format!(
                r#"{{"primary_db_url":":memory:","history_db_url":"http://{addr}","history_db_org":"t","history_db_token":"t"}}"#
            )
            .as_bytes(),
        )
        .unwrap();
    // `CONFIG_FILE` is process-global: the shared guard serializes the
    // mutation across tests and restores the previous value on drop.
    let guard = crate::test_env::ConfigFileGuard::acquire();
    guard.set(Some(config_path.to_str().unwrap()));

    // The real production constructor: the migration runner applies the
    // schema, including `sensor_battery`, on a fresh database.
    let connection = Connection::new().await.unwrap();
    // Parent sensor created through the regular service: a runner-created
    // database contains no sensor rows, and `record` needs a real sensor id
    // so the upsert passes the foreign-key constraint.
    let sensors = SensorService::new(connection.clone());
    let sensor_id = sensors.add("11:22:33:44:55:66").await.unwrap();
    // Trait-based construction path: `Service` must be in scope to resolve
    // `new`, exactly like every other service of the crate.
    let service = BatteryService::new(connection);

    let sample = BatteryReading {
        sensor_id,
        resting_mv: 4100,
        loaded_mv: 4050,
        received_at: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    };
    let recorded = service.record(&sample).await;
    assert!(
        matches!(recorded, Err(BatteryError::HistoryWrite(_))),
        "a failing history endpoint must surface as BatteryError::HistoryWrite, got {recorded:?}"
    );
    // The request reached the local fake (it answers only after reading the
    // body), so the failure provably came from the deterministic endpoint.
    server.join().unwrap();
    // The SQLite latest value was committed before the history write failed.
    assert_eq!(service.latest(sensor_id).await.unwrap(), Some(sample));

    // Drop the guard: it restores the previous CONFIG_FILE value.
    drop(guard);
    let _ = std::fs::remove_file(&config_path);
}
