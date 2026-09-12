use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::refresh_token::error::RotationOutcome;
use crate::refresh_token::repository::{session_digest, token_digest};
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

// Session-domain tests. Sessions share the refresh_tokens table but are
// stored under `grpc-session:`-tagged digests; these tests pin the public
// validation/revocation contract, the input validation, and the mandatory
// bidirectional separation between the session and refresh token domains.

// Raw-row helper for session rows: binds the domain-tagged digest the
// repository boundary stores (same hashing the production code applies).
async fn session_row_count(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<i64> {
    let row =
        sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ? AND token = ?")
            .bind(user_id)
            .bind(session_digest(token))
            .fetch_one(pool)
            .await?;
    Ok(row.try_get::<i64, _>("n")?)
}

// Simulates the clock moving past a session's expiry by rewriting its
// `expire_at` into the past (no real-time waiting in tests).
async fn expire_session(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<()> {
    sqlx::query("UPDATE refresh_tokens SET expire_at = ? WHERE user_id = ? AND token = ?")
        .bind(Utc::now() - Duration::hours(1))
        .bind(user_id)
        .bind(session_digest(token))
        .execute(pool)
        .await?;
    Ok(())
}

// Core session lifecycle through the public API: store, validate
// repeatedly (non-consuming), unknown values fail closed.
#[tokio::test]
async fn test_add_and_validate_session_non_consuming() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    // Non-consuming: the same presentation validates again.
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    // Unknown values return no user (fail closed, no error).
    assert_eq!(service.validate_session("unknown").await?, None);
    Ok(())
}

// At-rest guarantee through the public API: the stored row carries the
// tagged digest, the plaintext matches no row anywhere, and the public
// surface never returns a digest (validation yields only the user ID).
#[tokio::test]
async fn test_session_raw_token_absent_at_rest() -> Result<()> {
    use sha2::{Digest, Sha256};

    // Independent digest computation (sha2 directly, not the repository
    // helper) so the format assertion is not circular.
    fn digest_hex(value: &str) -> String {
        Sha256::digest(value.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    service
        .add_session(
            user_id,
            "session-raw",
            Utc::now() + Duration::hours(1),
            Utc::now(),
        )
        .await?;
    let rows = sqlx::query("SELECT token FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&*pool)
        .await?;
    assert_eq!(rows.len(), 1);
    let stored: String = rows[0].try_get("token")?;
    assert_eq!(
        stored,
        format!("grpc-session:{}", digest_hex("session-raw"))
    );
    let plaintext = sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE token = ?")
        .bind("session-raw")
        .fetch_one(&*pool)
        .await?;
    assert_eq!(plaintext.try_get::<i64, _>("n")?, 0);
    // Validation returns only the owning user ID, never stored material.
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    Ok(())
}

// Expiry semantics and the session cleanup sweep: an expired session stops
// validating, the cleanup removes its row, and the unexpired sibling (and
// every refresh row) survives.
#[tokio::test]
async fn test_expired_session_not_validated_and_swept_by_cleanup() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "live-session", future, Utc::now())
        .await?;
    service
        .add_session(user_id, "dead-session", future, Utc::now())
        .await?;
    expire_session(&pool, user_id, "dead-session").await?;
    assert_eq!(service.validate_session("dead-session").await?, None);
    // Cleanup removes exactly the expired session row.
    service.cleanup_expired_sessions().await?;
    assert_eq!(session_row_count(&pool, user_id, "dead-session").await?, 0);
    assert_eq!(session_row_count(&pool, user_id, "live-session").await?, 1);
    assert_eq!(
        service.validate_session("live-session").await?,
        Some(user_id)
    );
    Ok(())
}

// Single revocation through the public API: the presented session stops
// validating, its sibling keeps working, and logout stays idempotent.
#[tokio::test]
async fn test_revoke_session_single_and_idempotent() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "session-one", future, Utc::now())
        .await?;
    service
        .add_session(user_id, "session-two", future, Utc::now())
        .await?;
    service.revoke_session("session-one").await?;
    assert_eq!(service.validate_session("session-one").await?, None);
    assert_eq!(
        service.validate_session("session-two").await?,
        Some(user_id)
    );
    // Revoking again — or an unknown value — stays a no-op success.
    service.revoke_session("session-one").await?;
    service.revoke_session("never-stored").await?;
    assert_eq!(
        service.validate_session("session-two").await?,
        Some(user_id)
    );
    Ok(())
}

// Logout-everywhere is strictly per-user: another user's session is out
// of reach of the revoke-all statement.
#[tokio::test]
async fn test_revoke_all_sessions_user_isolation() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_a = user_service_add(&connection, "alice", "alice@example.com").await?;
    let user_b = user_service_add(&connection, "bob", "bob@example.com").await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_a, "a-session", future, Utc::now())
        .await?;
    service
        .add_session(user_b, "b-session", future, Utc::now())
        .await?;
    service.revoke_all_sessions(user_a).await?;
    assert_eq!(service.validate_session("a-session").await?, None);
    assert_eq!(service.validate_session("b-session").await?, Some(user_b));
    Ok(())
}

// Duplicate session insert: the typed AlreadyExists error, and the prior
// row is kept exactly as stored (no overwrite, no expiry extension).
#[tokio::test]
async fn test_duplicate_session_insert_yields_already_exists_without_overwrite() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let created_at = Utc::now();
    let expire_at = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "session", expire_at, created_at)
        .await?;
    let duplicate = service
        .add_session(
            user_id,
            "session",
            expire_at + Duration::hours(5),
            created_at,
        )
        .await;
    assert!(
        matches!(&duplicate, Err(RefreshTokenError::AlreadyExists)),
        "expected Err(RefreshTokenError::AlreadyExists), got: {duplicate:?}"
    );
    // Exactly one row, with the ORIGINAL expiry (read through the raw row
    // so the no-extension guarantee does not depend on the API surface).
    let row = sqlx::query("SELECT expire_at FROM refresh_tokens WHERE user_id = ? AND token = ?")
        .bind(user_id)
        .bind(session_digest("session"))
        .fetch_one(&*pool)
        .await?;
    let stored_expire: chrono::NaiveDateTime = row.try_get("expire_at")?;
    assert_eq!(stored_expire, expire_at.naive_utc());
    assert_eq!(service.validate_session("session").await?, Some(user_id));
    Ok(())
}

// Presented-value validation happens BEFORE any SQL: empty values are
// refused on every session method, one byte past the ceiling is refused,
// the exact ceiling is accepted, and an inverted validity window is
// refused at store time. Rejected inputs store nothing.
#[tokio::test]
async fn test_session_presented_values_rejected_before_sql() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let now = Utc::now();

    // Empty presented value: refused on every session method.
    assert!(
        matches!(
            service
                .add_session(user_id, "", now + Duration::hours(1), now)
                .await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "empty token must be refused on add"
    );
    assert!(
        matches!(
            service.validate_session("").await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "empty token must be refused on validate"
    );
    assert!(
        matches!(
            service.revoke_session("").await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "empty token must be refused on revoke"
    );

    // Oversized presented value: one byte past the ceiling is refused,
    // the exact ceiling is accepted (the boundary is inclusive).
    let oversized = "x".repeat(MAX_SESSION_TOKEN_BYTES + 1);
    assert!(
        matches!(
            service
                .add_session(user_id, &oversized, now + Duration::hours(1), now)
                .await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "oversized token must be refused"
    );
    let boundary = "x".repeat(MAX_SESSION_TOKEN_BYTES);
    service
        .add_session(user_id, &boundary, now + Duration::hours(1), now)
        .await?;
    assert_eq!(service.validate_session(&boundary).await?, Some(user_id));

    // Inverted validity window: refused at store time (equal and past).
    assert!(
        matches!(
            service.add_session(user_id, "window", now, now).await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "expire_at == created_at must be refused"
    );
    assert!(
        matches!(
            service
                .add_session(user_id, "window", now - Duration::hours(1), now)
                .await,
            Err(RefreshTokenError::Common(CommonError::Operation(_)))
        ),
        "expire_at < created_at must be refused"
    );

    // Exactly one session row reached SQL: the accepted boundary value.
    let row = sqlx::query(
        "SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ? AND token LIKE 'grpc-session:%'",
    )
    .bind(user_id)
    .fetch_one(&*pool)
    .await?;
    assert_eq!(row.try_get::<i64, _>("n")?, 1);
    Ok(())
}

// Serializable validate/revoke interleave: SQLite serializes writers, so
// a validation running against a concurrent logout either observes the
// row before the revocation commits (Some) or after it (None) — never a
// torn state — and the post-revocation world validates nothing.
#[tokio::test]
async fn test_concurrent_validate_and_revoke_serializable() -> Result<()> {
    let connection = Connection::new().await?;
    let service = std::sync::Arc::new(RefreshTokenService::new(connection.clone()));
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "session", future, Utc::now())
        .await?;

    let validate_service = service.clone();
    let revoke_service = service.clone();
    let (validated, revoked) = tokio::join!(
        async move { validate_service.validate_session("session").await },
        async move { revoke_service.revoke_session("session").await }
    );
    assert!(revoked.is_ok(), "revocation must succeed, got {revoked:?}");
    assert!(
        matches!(&validated, Ok(Some(id)) if *id == user_id) || matches!(validated, Ok(None)),
        "validation must observe a serializable pre- or post-revocation state, got {validated:?}"
    );
    // Once the revocation is visible, nothing validates anymore.
    assert_eq!(service.validate_session("session").await?, None);
    Ok(())
}

// Separation, direction 1: a session raw token cannot rotate as a REST
// refresh token (rotation matches only bare refresh digests), and the
// session survives the rotation attempt untouched.
#[tokio::test]
async fn test_session_token_cannot_rotate_as_refresh() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    service
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;
    let outcome = service
        .rotate(user_id, "session-raw", "replacement", future, Utc::now())
        .await?;
    assert_eq!(outcome, RotationOutcome::NotFound);
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    Ok(())
}

// Separation, direction 2: a refresh raw token cannot validate as a
// session, and it keeps rotating in its own domain.
#[tokio::test]
async fn test_refresh_token_cannot_validate_as_session() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(user_id, "refresh-raw", future, created_at)
        .await?;
    assert_eq!(service.validate_session("refresh-raw").await?, None);
    let outcome = service
        .rotate(user_id, "refresh-raw", "replacement", future, created_at)
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    Ok(())
}

// Separation, sweep 1: the add-path expiry purge (run on every refresh
// issuance) deletes expired refresh rows but never a live session row.
#[tokio::test]
async fn test_add_path_purge_leaves_unexpired_sessions_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(
            user_id,
            "expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;
    service
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;
    // The issuance triggers the add-path expired-only purge.
    service.add(user_id, "fresh", future, created_at).await?;
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    // Sanity: the expired refresh row really was purged by the issuance.
    let gone = service
        .rotate(user_id, "expired", "x", future, created_at)
        .await?;
    assert_eq!(gone, RotationOutcome::NotFound);
    Ok(())
}

// Separation, sweep 2: both purges inside the rotation transaction (the
// expired-only retention purge and the replay-driven reuse purge) leave
// live session rows intact.
#[tokio::test]
async fn test_rotation_purges_leave_unexpired_sessions_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service.add(user_id, "old", future, created_at).await?;
    service
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;
    // Legitimate rotation: the in-transaction retention purge runs.
    let rotated = service
        .rotate(user_id, "old", "winner", future, created_at)
        .await?;
    assert!(matches!(rotated, RotationOutcome::Rotated(_)));
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    // Replay: the reuse purge deletes every refresh row of the user.
    let replay = service
        .rotate(user_id, "old", "loser", future, created_at)
        .await?;
    assert_eq!(replay, RotationOutcome::ReuseDetected);
    // The session row is still live after both purges.
    assert_eq!(
        service.validate_session("session-raw").await?,
        Some(user_id)
    );
    Ok(())
}

// Separation, sweep 3: the global public cleanup_expired hook deletes
// expired refresh rows across all users but never a live session row.
#[tokio::test]
async fn test_public_cleanup_expired_leaves_unexpired_sessions_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_a = user_service_add(&connection, "alice", "alice@example.com").await?;
    let user_b = user_service_add(&connection, "bob", "bob@example.com").await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(
            user_a,
            "a-expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;
    service
        .add(
            user_b,
            "b-expired",
            Utc::now() - Duration::hours(1),
            created_at,
        )
        .await?;
    service
        .add_session(user_a, "a-session", future, Utc::now())
        .await?;
    service
        .add_session(user_b, "b-session", future, Utc::now())
        .await?;
    service.cleanup_expired().await?;
    // Expired refresh rows are gone; live session rows survive on both
    // users (the sweep is global).
    assert_eq!(service.validate_session("a-session").await?, Some(user_a));
    assert_eq!(service.validate_session("b-session").await?, Some(user_b));
    let a_gone = service
        .rotate(user_a, "a-expired", "x", future, created_at)
        .await?;
    assert_eq!(a_gone, RotationOutcome::NotFound);
    let b_gone = service
        .rotate(user_b, "b-expired", "x", future, created_at)
        .await?;
    assert_eq!(b_gone, RotationOutcome::NotFound);
    Ok(())
}

// Separation, reverse sweep: the session expiry cleanup deletes expired
// session rows while unexpired sessions AND refresh rows survive.
#[tokio::test]
async fn test_session_cleanup_leaves_unexpired_refresh_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(user_id, "refresh-raw", future, created_at)
        .await?;
    service
        .add_session(user_id, "live-session", future, Utc::now())
        .await?;
    service
        .add_session(user_id, "dead-session", future, Utc::now())
        .await?;
    expire_session(&pool, user_id, "dead-session").await?;
    service.cleanup_expired_sessions().await?;
    assert_eq!(service.validate_session("dead-session").await?, None);
    assert_eq!(
        service.validate_session("live-session").await?,
        Some(user_id)
    );
    // The refresh row is untouched by the session sweep and still rotates.
    let outcome = service
        .rotate(user_id, "refresh-raw", "replacement", future, created_at)
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    Ok(())
}

// Separation, logout scope: revoking every session of a user leaves the
// user's REST refresh rotation fully working.
#[tokio::test]
async fn test_session_revoke_all_leaves_refresh_rotation_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(user_id, "refresh-raw", future, created_at)
        .await?;
    service
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;
    service.revoke_all_sessions(user_id).await?;
    assert_eq!(service.validate_session("session-raw").await?, None);
    let outcome = service
        .rotate(user_id, "refresh-raw", "replacement", future, created_at)
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    Ok(())
}

// Separation, storage identity: the SAME raw value stored in both domains
// yields two distinct rows (bare digest vs tagged digest), each domain
// consumes/validates only its own row, and the duplicate constraint never
// fires across domains.
#[tokio::test]
async fn test_identical_raw_input_distinct_across_domains() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = RefreshTokenService::new(connection.clone());
    let user_id = test_user(&connection).await?;
    let future = Utc::now() + Duration::hours(1);
    let created_at = Utc::now();
    service
        .add(user_id, "same-raw-value", future, created_at)
        .await?;
    // No AlreadyExists: the tagged session digest differs from the bare
    // refresh digest, so both rows coexist.
    service
        .add_session(user_id, "same-raw-value", future, created_at)
        .await?;
    let rows = sqlx::query("SELECT token FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&*pool)
        .await?;
    assert_eq!(rows.len(), 2);
    let stored: Vec<String> = rows
        .iter()
        .map(|row| row.try_get("token"))
        .collect::<std::result::Result<_, _>>()?;
    assert!(stored.contains(&token_digest("same-raw-value")));
    assert!(stored.contains(&session_digest("same-raw-value")));
    // Each domain operates only on its own row: the refresh value rotates
    // in the refresh domain while the session value keeps validating.
    let outcome = service
        .rotate(user_id, "same-raw-value", "replacement", future, created_at)
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    assert_eq!(
        service.validate_session("same-raw-value").await?,
        Some(user_id)
    );
    Ok(())
}
