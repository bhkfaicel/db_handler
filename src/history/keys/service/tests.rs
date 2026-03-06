use crate::{
    db::connection::Connection,
    db::primary::service::Service,
    history::keys::{model::HistoricalKey, service::HistoricalKeyService},
};
use anyhow::Result;

#[tokio::test]
async fn test_add_historical_key() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = HistoricalKeyService::new(connection);
    let key = [26; 32];
    let historical_key = service.add(key).await?;
    let created_historical_key = sqlx::query_as::<_, HistoricalKey>(
        r#"
        SELECT * FROM historical_keys WHERE id = ?
        "#,
    )
    .bind(historical_key.id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created historical key");
    assert!(historical_key.id > 0);
    assert_eq!(created_historical_key, historical_key);
    Ok(())
}
