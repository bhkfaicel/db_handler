use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::users::service::UserService;
use anyhow::Result;
use chrono::{Duration, Utc};

#[tokio::test]
async fn test_add_refresh_token() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let token = "token";
    let expire_at = Utc::now();
    let created_at = Utc::now();
    let refresh_token = service.add(user_id, token, expire_at, created_at).await?;
    assert!(refresh_token.id > 0);
    assert_eq!(refresh_token.user_id, user_id);
    assert_eq!(refresh_token.token, token);
    assert_eq!(refresh_token.expire_at, expire_at);
    assert_eq!(refresh_token.created_at, created_at);
    assert_eq!(refresh_token.is_used, false);
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    service.delete_by_user_id(user_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    service.delete_by_user_id(2606).await?;
    Ok(())
}

#[tokio::test]
async fn test_is_valid() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut expire_at: DateTime<Utc> = Utc::now();
    expire_at = expire_at + Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    let token = "token";
    service.add(user_id, token, expire_at, created_at).await?;
    let result = service.is_valid(user_id, token).await?;
    assert_eq!(result, true);
    Ok(())
}

#[tokio::test]
async fn test_is_valid_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let result = service.is_valid(user_id, "token2").await;
    match result {
        Err(RefreshTokenError::NotFound) => {}
        other => panic!("Expected Err(RefreshTokenError::NotFound), got {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn test_is_valid_expired() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let mut expire_at: DateTime<Utc> = Utc::now();
    expire_at = expire_at - Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    let token = "token";
    service.add(user_id, token, expire_at, created_at).await?;
    let result = service.is_valid(user_id, "token").await;
    match result {
        Err(RefreshTokenError::Expired) => {}
        other => panic!("Expected Err(RefreshTokenError::Expired), got {:?}", other),
    }
    Ok(())
}
