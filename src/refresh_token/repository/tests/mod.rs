use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::refresh_token::model::RefreshToken;
use crate::users::service::UserService;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

#[tokio::test]
async fn test_insert_refresh_token() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let mut refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: true,
    };
    let id = repository.insert(&refresh_token).await?;
    refresh_token.id = id;
    refresh_token.is_used = false;
    let retrieved_refresh_token = repository
        .get_by_user_id_and_token(user_id, &refresh_token.token)
        .await?;
    assert!(retrieved_refresh_token.is_some());
    let retrieved_refresh_token = retrieved_refresh_token.unwrap();
    assert_eq!(retrieved_refresh_token, refresh_token);
    Ok(())
}

#[tokio::test]
async fn test_insert_refresh_token_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let refresh_token = RefreshToken {
        id: 0,
        user_id: 2606,
        token: "token".to_string(),
        expire_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
        is_used: false,
    };
    let id = repository.insert(&refresh_token).await;
    assert!(id.is_err());
    Ok(())
}

#[tokio::test]
async fn test_get_by_user_id_and_token() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let mut refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let id = repository.insert(&refresh_token).await?;
    refresh_token.id = id;
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token.token)
        .await?;
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result, refresh_token);
    Ok(())
}

#[tokio::test]
async fn test_get_by_user_id_and_token_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let result = repository.get_by_user_id_and_token(1, "token").await?;
    assert!(result.is_none());
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let refresh_token2 = RefreshToken {
        id: 0,
        user_id,
        token: "token2".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let _ = repository.insert(&refresh_token).await?;
    let _ = repository.insert(&refresh_token2).await?;
    repository.delete_by_user_id(user_id as u32).await?;
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token.token)
        .await?;
    assert!(result.is_none());
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token2.token)
        .await?;
    assert!(result.is_none());
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let mut refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let id = repository.insert(&refresh_token).await?;
    refresh_token.id = id;
    repository.delete_by_user_id(2606).await?;
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token.token)
        .await?;
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result, refresh_token);
    Ok(())
}

#[tokio::test]
async fn test_update_flag_by_token() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    repository.update_flag_by_token("token").await?;
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token.token)
        .await?;
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.is_used);
    Ok(())
}

#[tokio::test]
async fn test_update_flag_by_token_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    repository.update_flag_by_token("token").await?;
    Ok(())
}

#[tokio::test]
async fn test_update_flag_by_token_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    repository.update_flag_by_token("token").await?;
    Ok(())
}

// After the migration that drops the redundant manual index on `token`,
// the UNIQUE column constraint's implicit index must keep serving every
// token-based path: both lookups must resolve, and the query planner must
// answer a `WHERE token = ?` predicate with an index search (the implicit
// unique index), never a full table scan.
#[tokio::test]
async fn test_token_paths_resolve_and_stay_indexed_after_duplicate_index_drop() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut now: DateTime<Utc> = Utc::now();
    now += Duration::hours(1);
    let mut refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let id = repository.insert(&refresh_token).await?;
    refresh_token.id = id;
    // Lookup by user id and token still resolves after the index drop.
    let result = repository
        .get_by_user_id_and_token(user_id as u32, &refresh_token.token)
        .await?;
    assert_eq!(result, Some(refresh_token));
    // Flag update by token still resolves and stays observable.
    repository.update_flag_by_token("token").await?;
    let result = repository
        .get_by_user_id_and_token(user_id as u32, "token")
        .await?;
    assert!(result.is_some());
    assert!(result.unwrap().is_used);
    // The query planner must serve the token predicate through the implicit
    // unique index created by the UNIQUE column constraint.
    let rows = sqlx::query("EXPLAIN QUERY PLAN SELECT * FROM refresh_tokens WHERE token = ?")
        .bind("token")
        .fetch_all(&*pool)
        .await?;
    let details: Vec<String> = rows
        .iter()
        .map(|row| row.try_get::<String, _>("detail"))
        .collect::<std::result::Result<_, _>>()?;
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("USING INDEX sqlite_autoindex_refresh_tokens")),
        "expected the UNIQUE constraint's implicit index in the query plan, got: {details:?}"
    );
    Ok(())
}
