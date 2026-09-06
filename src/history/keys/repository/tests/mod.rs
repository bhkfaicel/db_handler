use chrono::{DateTime, Duration, Utc};

use crate::db::connection::Connection;

use super::*;

// The parent module now re-exports the `CommonError`-based `Result` alias
// through `use super::*`; pin anyhow's aliases explicitly so the test
// returns `anyhow::Result` and both error carriages convert through `?`.
use anyhow::{Ok, Result};

#[tokio::test]
async fn test_insert_historical_key() -> Result<()> {
    let connection = Connection::new().await?;
    let historical_repository = HistoricalKeyRepository::new(connection.primary_db_client());
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let key = [26; 32];
    let historical_key = HistoricalKey {
        id: 0,
        value: key.clone().to_vec(),
        expire_at: now,
    };
    let id = historical_repository.insert(&historical_key).await?;
    let created_historical_key = sqlx::query_as::<_, HistoricalKey>(
        r#"
        SELECT * FROM historical_keys WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*connection.primary_db_client())
    .await
    .expect("Failed to fetch created historical key");
    assert!(id > 0);
    assert_eq!(created_historical_key.id, id);
    assert_eq!(created_historical_key.expire_at, now);
    assert_eq!(created_historical_key.value, key.clone());
    Ok(())
}
