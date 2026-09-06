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
    assert!(!refresh_token.is_used);
    Ok(())
}

/// Typed non-regression test: adding a second refresh token carrying a
/// token value that is already stored must yield the dedicated
/// `AlreadyExists` variant, mapped from the structured unique-violation
/// constraint (`refresh_tokens.token`) reported by the repository.
#[tokio::test]
async fn add_duplicate_token_yields_already_exists() -> Result<()> {
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
    let expire_at = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service.add(user_id, "token", expire_at, created_at).await?;
    let duplicate = service.add(user_id, "token", expire_at, created_at).await;
    assert!(
        matches!(&duplicate, Err(RefreshTokenError::AlreadyExists)),
        "expected Err(RefreshTokenError::AlreadyExists), got: {duplicate:?}"
    );
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
    expire_at += Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    let token = "token";
    service.add(user_id, token, expire_at, created_at).await?;
    let result = service.is_valid(user_id, token).await?;
    assert!(result);
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
    expire_at -= Duration::hours(1);
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

/// Token issuance doubles as the retention trigger for its own user: the
/// expired and already-used tokens are purged before the new token is
/// inserted, so per-user growth stays bounded to live tokens.
#[tokio::test]
async fn test_add_purges_expired_and_used_tokens() -> Result<()> {
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

    // Seed one expired token and one already-used token for the user.
    service
        .add(
            user_id,
            "expired",
            Utc::now() - Duration::hours(1),
            Utc::now(),
        )
        .await?;
    service
        .add(user_id, "used", Utc::now() + Duration::hours(1), Utc::now())
        .await?;
    service.update_flag_by_token("used").await?;

    // Issuing a new token must purge the expired and used rows first.
    service
        .add(
            user_id,
            "fresh",
            Utc::now() + Duration::hours(1),
            Utc::now(),
        )
        .await?;

    // Purged rows are gone (NotFound, not Expired/AlreadyUsed).
    let expired_result = service.is_valid(user_id, "expired").await;
    assert!(
        matches!(expired_result, Err(RefreshTokenError::NotFound)),
        "the expired token must be purged on issuance, got {expired_result:?}"
    );
    let used_result = service.is_valid(user_id, "used").await;
    assert!(
        matches!(used_result, Err(RefreshTokenError::NotFound)),
        "the used token must be purged on issuance, got {used_result:?}"
    );
    // The newly issued token remains valid.
    assert!(service.is_valid(user_id, "fresh").await?);
    Ok(())
}

/// The public boot-time sweep hook: expired tokens of users who never log
/// in again are only removed by this explicit caller-driven call, while
/// valid rows must survive it.
#[tokio::test]
async fn test_cleanup_expired_sweeps_expired_tokens_globally() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_service = UserService::new(connection.clone());
    let user_a = user_service
        .add(
            "alice".to_string(),
            "alice@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let user_b = user_service
        .add(
            "bob".to_string(),
            "bob@example.com".to_string(),
            "password".to_string(),
        )
        .await?;

    // User A holds one expired and one valid token; user B only an expired
    // one (simulating a user who never logs in again).
    service
        .add(
            user_a,
            "a-expired",
            Utc::now() - Duration::hours(1),
            Utc::now(),
        )
        .await?;
    service
        .add(
            user_a,
            "a-valid",
            Utc::now() + Duration::hours(1),
            Utc::now(),
        )
        .await?;
    service
        .add(
            user_b,
            "b-expired",
            Utc::now() - Duration::hours(1),
            Utc::now(),
        )
        .await?;

    service.cleanup_expired().await?;

    let a_expired = service.is_valid(user_a, "a-expired").await;
    assert!(
        matches!(a_expired, Err(RefreshTokenError::NotFound)),
        "user A's expired token must be swept, got {a_expired:?}"
    );
    let b_expired = service.is_valid(user_b, "b-expired").await;
    assert!(
        matches!(b_expired, Err(RefreshTokenError::NotFound)),
        "user B's expired token must be swept, got {b_expired:?}"
    );
    assert!(service.is_valid(user_a, "a-valid").await?);
    Ok(())
}
