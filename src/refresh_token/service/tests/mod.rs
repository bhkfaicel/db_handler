use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::refresh_token::error::RotationOutcome;
use crate::refresh_token::repository::token_digest;
use crate::users::service::UserService;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

// Runtime-SQL helpers (plain `sqlx::query`, not the compile-time macro: the
// runtime API needs no offline `.sqlx` artifact). They back the raw row
// assertions of the retention/bounded-storage tests. They bind the SHA-256
// digest of the caller-side token: at rest only digests are stored, so the
// assertions reuse the same boundary hashing the repository performs.
async fn token_row_count(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<i64> {
    let row =
        sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ? AND token = ?")
            .bind(user_id)
            .bind(token_digest(token))
            .fetch_one(pool)
            .await?;
    Ok(row.try_get::<i64, _>("n")?)
}

// Simulates the clock moving past a token's expiry by rewriting its
// `expire_at` into the past (no real-time waiting in tests).
async fn expire_token(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<()> {
    sqlx::query("UPDATE refresh_tokens SET expire_at = ? WHERE user_id = ? AND token = ?")
        .bind(Utc::now() - Duration::hours(1))
        .bind(user_id)
        .bind(token_digest(token))
        .execute(pool)
        .await?;
    Ok(())
}

// Every test needs one owning user (the table's user_id foreign key).
async fn test_user(connection: &Connection) -> Result<u32> {
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(
            "username".to_string(),
            "email@example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    Ok(user_id)
}

#[tokio::test]
async fn test_add_refresh_token() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
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
    let user_id = test_user(&connection).await?;
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
    let user_id = test_user(&connection).await?;
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

// A live token rotates: the outcome carries the inserted replacement.
#[tokio::test]
async fn test_rotate_returns_rotated_with_replacement() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let expire_at: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    service.add(user_id, "token", expire_at, created_at).await?;
    let outcome = service
        .rotate(user_id, "token", "rotated", expire_at, created_at)
        .await?;
    match &outcome {
        RotationOutcome::Rotated(rotated) => {
            assert!(rotated.id > 0);
            assert_eq!(rotated.user_id, user_id);
            assert_eq!(rotated.token, "rotated");
        }
        other => panic!("expected Rotated, got {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn test_rotate_unknown_token_yields_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let expire_at: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    let outcome = service
        .rotate(user_id, "token2", "replacement", expire_at, created_at)
        .await?;
    assert_eq!(outcome, RotationOutcome::NotFound);
    Ok(())
}

// Hashing contract, end to end through the public API: a token inserted
// via `add` is consumable by `rotate` with the caller-side string, and the
// replacement is consumable in turn (store and lookup paths hash
// consistently). The rotated model echoes the caller-supplied value, never
// a stored digest.
#[tokio::test]
async fn test_insert_then_rotate_with_caller_token_succeeds() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(user_id, "caller-token", future, created_at)
        .await?;
    let first = service
        .rotate(user_id, "caller-token", "second-token", future, created_at)
        .await?;
    match &first {
        RotationOutcome::Rotated(rotated) => {
            // Input echo: the caller-supplied replacement value, not the
            // digest the database stores.
            assert_eq!(rotated.token, "second-token");
        }
        other => panic!("expected Rotated, got {other:?}"),
    }
    // The replacement itself rotates: the digest stored by the first
    // rotation is found again from the caller-side string.
    let second = service
        .rotate(user_id, "second-token", "third-token", future, created_at)
        .await?;
    assert!(matches!(second, RotationOutcome::Rotated(_)));
    Ok(())
}

#[tokio::test]
async fn test_rotate_expired_token_yields_expired() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let expired_at: DateTime<Utc> = Utc::now() - Duration::hours(1);
    let created_at: DateTime<Utc> = Utc::now();
    service
        .add(user_id, "token", expired_at, created_at)
        .await?;
    let outcome = service
        .rotate(user_id, "token", "replacement", expired_at, created_at)
        .await?;
    assert_eq!(outcome, RotationOutcome::Expired);
    Ok(())
}

// Issuance purge under the expired-only policy: expired rows are removed,
// while an already-used but unexpired token is deliberately retained (its
// replay must keep classifying as reuse for the token's full lifetime).
#[tokio::test]
async fn test_add_purges_only_expired_tokens_and_keeps_used() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();

    // Seed one expired token and consume a second one.
    service
        .add(
            user_id,
            "expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;
    service.add(user_id, "used", future, created_at).await?;
    let consumed = service
        .rotate(user_id, "used", "used-rotated", future, created_at)
        .await?;
    assert!(
        matches!(consumed, RotationOutcome::Rotated(_)),
        "seeding: the second token must rotate cleanly, got {consumed:?}"
    );

    // Issuing a new token runs the expired-only purge.
    service.add(user_id, "fresh", future, created_at).await?;

    // The expired row is gone (purged on issuance).
    let expired_result = service
        .rotate(user_id, "expired", "x", future, created_at)
        .await?;
    assert_eq!(expired_result, RotationOutcome::NotFound);
    // The newly issued token remains live (asserted BEFORE the replay
    // below: reuse detection deletes every token of the user).
    let fresh_result = service
        .rotate(user_id, "fresh", "x", future, created_at)
        .await?;
    assert!(matches!(fresh_result, RotationOutcome::Rotated(_)));
    // The used-but-unexpired row is RETAINED: its replay still classifies
    // as reuse (this is the deliberate policy change from purging used
    // rows on issuance). Runs last: the reuse purge wipes the user's
    // remaining tokens.
    let used_result = service
        .rotate(user_id, "used", "x", future, created_at)
        .await?;
    assert_eq!(used_result, RotationOutcome::ReuseDetected);
    Ok(())
}

// The public boot-time sweep hook: expired tokens of users who never log
// in again are only removed by this explicit caller-driven call, while
// valid rows must survive it.
#[tokio::test]
async fn test_cleanup_expired_sweeps_expired_tokens_globally() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_a = user_service_add(&connection, "alice", "alice@example.com").await?;
    let user_b = user_service_add(&connection, "bob", "bob@example.com").await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();

    // User A holds one expired and one valid token; user B only an expired
    // one (simulating a user who never logs in again).
    service
        .add(
            user_a,
            "a-expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;
    service.add(user_a, "a-valid", future, created_at).await?;
    service
        .add(
            user_b,
            "b-expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;

    service.cleanup_expired().await?;

    // Swept tokens classify as not found; the valid token still rotates.
    let a_expired = service
        .rotate(user_a, "a-expired", "x", future, created_at)
        .await?;
    assert_eq!(a_expired, RotationOutcome::NotFound);
    let b_expired = service
        .rotate(user_b, "b-expired", "x", future, created_at)
        .await?;
    assert_eq!(b_expired, RotationOutcome::NotFound);
    let a_valid = service
        .rotate(user_a, "a-valid", "x", future, created_at)
        .await?;
    assert!(matches!(a_valid, RotationOutcome::Rotated(_)));
    Ok(())
}

// Concurrency ordering, winner-first: the winner's rotation commits before
// the loser's, so the loser's presentation of the same token classifies as
// reuse and its purge deletes the winner's replacement; a subsequent
// rotation with that replacement finds nothing.
#[tokio::test]
async fn test_concurrent_rotation_winner_first_loser_gets_reuse_detected() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service.add(user_id, "old", future, created_at).await?;

    // Winner commits first: the token is consumed and replaced.
    let winner = service
        .rotate(user_id, "old", "winner", future, created_at)
        .await?;
    assert!(
        matches!(winner, RotationOutcome::Rotated(_)),
        "winner must rotate first, got {winner:?}"
    );
    // Loser replays the same token: reuse detected, all user tokens
    // (including the winner's replacement) are deleted.
    let loser = service
        .rotate(user_id, "old", "loser", future, created_at)
        .await?;
    assert_eq!(loser, RotationOutcome::ReuseDetected);
    // The winner's replacement was deleted by the loser's purge.
    let after = service
        .rotate(user_id, "winner", "next", future, created_at)
        .await?;
    assert_eq!(after, RotationOutcome::NotFound);
    Ok(())
}

// Concurrency ordering, loser-first: the reuse purge commits before the
// winner's consumption, so the winner's UPDATE matches zero rows and no
// replacement is inserted (a half-purged, half-replaced state is
// impossible).
#[tokio::test]
async fn test_concurrent_rotation_loser_first_winner_inserts_nothing() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service.add(user_id, "old", future, created_at).await?;

    // A legitimate rotation consumes the token first.
    let first = service
        .rotate(user_id, "old", "first", future, created_at)
        .await?;
    assert!(
        matches!(first, RotationOutcome::Rotated(_)),
        "seeding rotation must succeed, got {first:?}"
    );
    // The reuse purge commits BEFORE the would-be winner runs.
    let loser = service
        .rotate(user_id, "old", "loser", future, created_at)
        .await?;
    assert_eq!(loser, RotationOutcome::ReuseDetected);
    // The winner's consumption now matches zero rows: NotFound, and no
    // replacement row is inserted.
    let winner = service
        .rotate(user_id, "old", "would-be-winner", future, created_at)
        .await?;
    assert_eq!(winner, RotationOutcome::NotFound);
    assert_eq!(token_row_count(&pool, user_id, "would-be-winner").await?, 0);
    assert_eq!(token_row_count(&pool, user_id, "old").await?, 0);
    Ok(())
}

// Retention policy: a consumed token is retained until its natural expiry;
// once that passes, the next issuance purge removes it.
#[tokio::test]
async fn test_used_token_purged_once_expired_by_next_issuance() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();

    service.add(user_id, "used", future, created_at).await?;
    let rotated = service
        .rotate(user_id, "used", "rotated", future, created_at)
        .await?;
    assert!(
        matches!(rotated, RotationOutcome::Rotated(_)),
        "seeding rotation must succeed, got {rotated:?}"
    );
    // While unexpired, the consumed row survives issuance purges.
    service.add(user_id, "fresh", future, created_at).await?;
    assert_eq!(token_row_count(&pool, user_id, "used").await?, 1);
    // Simulate the clock passing the consumed token's expiry.
    expire_token(&pool, user_id, "used").await?;
    // The next issuance purge removes it.
    service.add(user_id, "fresh2", future, created_at).await?;
    let gone = service
        .rotate(user_id, "used", "x", future, created_at)
        .await?;
    assert_eq!(gone, RotationOutcome::NotFound);
    Ok(())
}

// Storage bound: rotations accumulate rows only within the expiry window.
// After k rotations whose consumed tokens all expired, one purge bounds the
// per-user storage to the unexpired rows instead of the rotation count.
#[tokio::test]
async fn test_repeated_rotations_stay_storage_bounded() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(2);
    let created_at = Utc::now();

    service.add(user_id, "t0", future, created_at).await?;
    // Chain of rotations: every consumed token is retained (expired-only
    // policy), so rows accumulate inside the expiry window.
    for (old, new) in [("t0", "t1"), ("t1", "t2"), ("t2", "t3")] {
        let outcome = service
            .rotate(user_id, old, new, future, created_at)
            .await?;
        assert!(
            matches!(outcome, RotationOutcome::Rotated(_)),
            "rotation {old} -> {new} must succeed, got {outcome:?}"
        );
    }
    // Four rows so far: t0, t1, t2 consumed + t3 live.
    assert_eq!(token_row_count_total(&pool, user_id).await?, 4);
    // The consumed tokens reach their expiry.
    for token in ["t0", "t1", "t2"] {
        expire_token(&pool, user_id, token).await?;
    }
    // One more rotation purges every expired consumed row: storage is
    // bounded by the expiry window (two rows), not by the rotation count.
    let outcome = service
        .rotate(user_id, "t3", "t4", future, created_at)
        .await?;
    assert!(
        matches!(outcome, RotationOutcome::Rotated(_)),
        "final rotation must succeed, got {outcome:?}"
    );
    assert_eq!(token_row_count_total(&pool, user_id).await?, 2);
    Ok(())
}

async fn token_row_count_total(pool: &sqlx::SqlitePool, user_id: u32) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get::<i64, _>("n")?)
}

// Convenience wrapper for tests needing a user with a distinct identity.
async fn user_service_add(connection: &Connection, name: &str, email: &str) -> Result<u32> {
    let user_service = UserService::new(connection.clone());
    let user_id = user_service
        .add(name.to_string(), email.to_string(), "password".to_string())
        .await?;
    Ok(user_id)
}
