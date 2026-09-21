use super::*;
use chrono::DateTime;
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
