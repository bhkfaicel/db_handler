use crate::{
    db::connection::Connection,
    db::primary::repository::PrimaryDbRepository,
    sensors::{model::Sensor, repository::SensorRepository},
};
use anyhow::{Ok, Result};
use sqlx::Row;

/// Counts the rows of `table` in the primary database.
async fn row_count(pool: &sqlx::SqlitePool, table: &str) -> i64 {
    // Table names are compile-time constants inside this test module only.
    sqlx::query(&format!("SELECT COUNT(*) AS n FROM {table}"))
        .fetch_one(pool)
        .await
        .expect("row count")
        .get::<i64, _>("n")
}

/// Seeds one sensor plus every child row that references it
/// (open_close_sensors state, key material, nonce).
async fn seed_sensor_with_children(repo: &SensorRepository, pool: &sqlx::SqlitePool, mac: &str) {
    repo.insert(&Sensor::new(
        0,
        mac.to_string(),
        crate::sensors::model::Status::Installed,
    ))
    .await
    .expect("sensor insert");
    let id = repo
        .get_by_mac(mac)
        .await
        .expect("sensor lookup")
        .expect("sensor present")
        .id;

    // keys.value references historical_keys.value: seed the parent first,
    // then reference exactly that same blob from the child row.
    let key_value: &[u8] = &[1u8, 2, 3, 4];
    sqlx::query("INSERT INTO historical_keys (value, expire_at) VALUES (?, datetime('now'))")
        .bind(key_value)
        .execute(pool)
        .await
        .expect("historical key insert");

    sqlx::query(
        "INSERT INTO open_close_sensors (sensor_id, sensor_mac, is_open, updated_at) \
         VALUES (?, ?, 0, datetime('now'))",
    )
    .bind(id)
    .bind(mac)
    .execute(pool)
    .await
    .expect("open/close state insert");

    sqlx::query("INSERT INTO keys (value, expire_at, mac) VALUES (?, datetime('now'), ?)")
        .bind(key_value)
        .bind(mac)
        .execute(pool)
        .await
        .expect("key insert");

    sqlx::query("INSERT INTO nonces (value, mac) VALUES (?, ?)")
        .bind([5u8, 6].as_slice())
        .bind(mac)
        .execute(pool)
        .await
        .expect("nonce insert");
}

/// Deleting by MAC must purge the open/close state row and cascade through
/// keys and nonces; before the CASCADE migration this failed with a foreign
/// key constraint error.
#[tokio::test]
async fn delete_by_mac_purges_every_child() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repo = SensorRepository::new(pool.clone());
    let mac = "00:1B:44:11:3A:D1";
    seed_sensor_with_children(&repo, &pool, mac).await;

    assert_eq!(row_count(&pool, "open_close_sensors").await, 1);
    assert_eq!(row_count(&pool, "keys").await, 1);
    assert_eq!(row_count(&pool, "nonces").await, 1);

    repo.delete_by_mac(mac).await?;

    assert_eq!(row_count(&pool, "sensors").await, 0);
    assert_eq!(row_count(&pool, "open_close_sensors").await, 0);
    assert_eq!(row_count(&pool, "keys").await, 0);
    assert_eq!(row_count(&pool, "nonces").await, 0);
    Ok(())
}

/// The by-id deletion path cascades exactly like the by-MAC path.
#[tokio::test]
async fn delete_by_id_purges_every_child() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repo = SensorRepository::new(pool.clone());
    let mac = "00:1B:44:11:3A:D2";
    seed_sensor_with_children(&repo, &pool, mac).await;
    let id = repo.get_by_mac(mac).await?.unwrap().id;

    repo.delete_by_id(id).await?;

    assert_eq!(row_count(&pool, "sensors").await, 0);
    assert_eq!(row_count(&pool, "open_close_sensors").await, 0);
    assert_eq!(row_count(&pool, "keys").await, 0);
    assert_eq!(row_count(&pool, "nonces").await, 0);
    Ok(())
}
