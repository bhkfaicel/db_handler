use super::*;
use crate::db::connection::Connection;
use crate::db::primary::service::Service;
use crate::refresh_token::error::RotationOutcome;
use crate::refresh_token::model::RefreshToken;
use crate::users::service::UserService;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

// Runtime-SQL helpers (plain `sqlx::query`, not the compile-time macro: the
// runtime API needs no offline `.sqlx` artifact). They back the raw row
// assertions the purge/replay policy tests rely on. They bind the SHA-256
// digest of the caller-side token: at rest only digests are stored, so
// asserting raw row state requires the same hashing the repository boundary
// performs (`token_digest`, in scope from the parent module).
async fn token_row_count(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<i64> {
    let row =
        sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ? AND token = ?")
            .bind(user_id)
            .bind(token_digest(token))
            .fetch_one(pool)
            .await?;
    Ok(row.try_get::<i64, _>("n")?)
}

async fn token_is_used(pool: &sqlx::SqlitePool, user_id: u32, token: &str) -> Result<Option<bool>> {
    let row = sqlx::query("SELECT is_used FROM refresh_tokens WHERE user_id = ? AND token = ?")
        .bind(user_id)
        .bind(token_digest(token))
        .fetch_optional(pool)
        .await?;
    match row {
        None => Ok(None),
        Some(row) => Ok(Some(row.try_get::<bool, _>("is_used")?)),
    }
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
async fn test_insert_refresh_token() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
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
    // Insert forces is_used to false regardless of the model value.
    let stored_is_used = token_is_used(&pool, user_id, &refresh_token.token).await?;
    assert_eq!(stored_is_used, Some(false));
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
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

// The rotation classification looks rows up by the (user_id, token) pair;
// these raw-SQL tests pin that lookup predicate: an inserted token is found
// for its owning user with the unused flag, and unknown pairs match nothing.
#[tokio::test]
async fn test_token_lookup_by_user_and_token() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(false));
    Ok(())
}

#[tokio::test]
async fn test_token_lookup_by_user_and_token_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    // A nonexistent user never matches another user's token (2606 is the
    // established nonexistent-user sentinel in this suite).
    assert_eq!(token_row_count(&pool, 2606, "token").await?, 0);
    // A different token never matches for the owning user.
    assert_eq!(token_row_count(&pool, user_id, "token2").await?, 0);
    Ok(())
}

// At-rest guarantee: the token column stores the SHA-256 hex digest, never
// the plaintext. The expected digest is computed INDEPENDENTLY here (sha2
// directly, not the repository helper) so the assertion is not circular.
#[tokio::test]
async fn test_token_stored_as_sha256_digest_not_plaintext() -> Result<()> {
    use sha2::{Digest, Sha256};

    // Independent digest computation for the at-rest assertion.
    fn digest_hex(value: &str) -> String {
        Sha256::digest(value.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    // Raw SQL over every stored row of the user: exactly one row, its
    // token column is the 64-character lowercase hex digest of the
    // plaintext, and the raw plaintext is absent (automated assertion, no
    // manual DB check).
    let rows = sqlx::query("SELECT token FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&*pool)
        .await?;
    assert_eq!(rows.len(), 1);
    let stored: String = rows[0].try_get("token")?;
    assert_ne!(stored, "token");
    assert_eq!(stored.len(), 64);
    assert!(
        stored
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "expected 64 lowercase hex chars, got {stored:?}"
    );
    assert_eq!(stored, digest_hex("token"));
    // The plaintext token matches no stored row.
    let plaintext_rows = sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE token = ?")
        .bind("token")
        .fetch_one(&*pool)
        .await?;
    assert_eq!(plaintext_rows.try_get::<i64, _>("n")?, 0);

    // Rotation stores the replacement under its digest too, and the
    // consumed token keeps its digest for replay detection (both rows are
    // digests; the plaintext appears nowhere).
    let outcome = repository
        .rotate(user_id, "token", &new_replacement(user_id, now), Utc::now())
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    let rows = sqlx::query("SELECT token FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&*pool)
        .await?;
    let stored: Vec<String> = rows
        .iter()
        .map(|row| row.try_get("token"))
        .collect::<std::result::Result<_, _>>()?;
    assert_eq!(stored.len(), 2);
    assert!(stored.contains(&digest_hex("token")));
    assert!(stored.contains(&digest_hex("replacement")));
    Ok(())
}

// Presenting the raw digest string AS IF it were the token must fail: the
// boundary hashes whatever it receives, so the digest-of-digest matches
// nothing and the real row stays unconsumed.
#[tokio::test]
async fn test_rotate_with_raw_digest_as_token_fails() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    // The digest string a database thief would find in the stolen dump.
    let raw_digest = token_digest("token");
    let outcome = repository
        .rotate(
            user_id,
            &raw_digest,
            &new_replacement(user_id, now),
            Utc::now(),
        )
        .await?;
    assert_eq!(outcome, RotationOutcome::NotFound);
    // The real token row is untouched: still present, still unused.
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(false));
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
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
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 0);
    assert_eq!(token_row_count(&pool, user_id, "token2").await?, 0);
    Ok(())
}

#[tokio::test]
async fn test_delete_by_user_id_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    // Deleting a nonexistent user leaves the real user's token untouched.
    repository.delete_by_user_id(2606).await?;
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(false));
    Ok(())
}

// Atomic rotation, happy path: the old token is consumed (kept, flagged
// used) and the replacement inserted in the same transaction.
#[tokio::test]
async fn test_rotate_consumes_and_replaces_atomically() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let old = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let replacement = RefreshToken {
        id: 0,
        user_id,
        token: "replacement".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&old).await?;
    let outcome = repository
        .rotate(user_id, "token", &replacement, Utc::now())
        .await?;
    match &outcome {
        RotationOutcome::Rotated(rotated) => {
            assert!(rotated.id > 0);
            assert_eq!(rotated.token, "replacement");
            assert_eq!(rotated.user_id, user_id);
            assert!(!rotated.is_used);
        }
        other => panic!("expected Rotated, got {other:?}"),
    }
    // The consumed token survives, flagged used (replay detection keeps
    // working for the token's full lifetime).
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(true));
    // The replacement is stored, unused.
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 1);
    assert_eq!(
        token_is_used(&pool, user_id, "replacement").await?,
        Some(false)
    );
    Ok(())
}

#[tokio::test]
async fn test_rotate_unknown_token_yields_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let replacement = RefreshToken {
        id: 0,
        user_id,
        token: "replacement".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let outcome = repository
        .rotate(user_id, "token", &replacement, Utc::now())
        .await?;
    assert_eq!(outcome, RotationOutcome::NotFound);
    // Nothing was inserted for a failed consumption.
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 0);
    Ok(())
}

#[tokio::test]
async fn test_rotate_unknown_user_yields_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let replacement = RefreshToken {
        id: 0,
        user_id,
        token: "replacement".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    // Unknown user_id: the classification must be NotFound, not an error.
    let outcome = repository
        .rotate(2606, "token", &replacement, Utc::now())
        .await?;
    assert_eq!(outcome, RotationOutcome::NotFound);
    Ok(())
}

// An expired token must classify as Expired and consume nothing: the
// replacement is not inserted, and the expired row itself is removed by the
// rotation's expired-only purge at the end of the same transaction (the
// classification is the proof the token was never consumed).
#[tokio::test]
async fn test_rotate_expired_token_yields_expired_and_inserts_nothing() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let expired = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: Utc::now() - Duration::hours(1),
        created_at: Utc::now(),
        is_used: false,
    };
    repository.insert(&expired).await?;
    let outcome = repository
        .rotate(
            user_id,
            "token",
            &new_replacement(user_id, now_plus_1h()),
            Utc::now(),
        )
        .await?;
    assert_eq!(outcome, RotationOutcome::Expired);
    // No replacement row was inserted for a failed consumption.
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 0);
    // The expired row was purged by the rotation's expired-only purge.
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 0);
    Ok(())
}

// Replay before expiry: the second presentation of a consumed token
// classifies as reuse and deletes EVERY token of the user (the winner's
// replacement included).
#[tokio::test]
async fn test_rotate_reuse_detected_deletes_all_user_tokens() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let old = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let other = RefreshToken {
        id: 0,
        user_id,
        token: "other".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&old).await?;
    repository.insert(&other).await?;
    let first = repository
        .rotate(user_id, "token", &new_replacement(user_id, now), Utc::now())
        .await?;
    assert!(matches!(first, RotationOutcome::Rotated(_)));
    // Replay of the now-consumed token, before its expiry.
    let replay = repository
        .rotate(user_id, "token", &new_replacement(user_id, now), Utc::now())
        .await?;
    assert_eq!(replay, RotationOutcome::ReuseDetected);
    // All the user's tokens are gone: the original, the sibling and the
    // winner's replacement.
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 0);
    assert_eq!(token_row_count(&pool, user_id, "other").await?, 0);
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 0);
    Ok(())
}

// Expired-only purge policy, inside the rotation transaction: the
// just-consumed token (expire_at in the future) SURVIVES for replay
// detection while already-expired rows of the same user are removed.
#[tokio::test]
async fn test_rotate_purge_keeps_consumed_token_and_removes_expired() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let old = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    let stale = RefreshToken {
        id: 0,
        user_id,
        token: "stale".to_string(),
        expire_at: Utc::now() - Duration::hours(1),
        created_at: Utc::now(),
        is_used: false,
    };
    repository.insert(&old).await?;
    repository.insert(&stale).await?;
    let outcome = repository
        .rotate(user_id, "token", &new_replacement(user_id, now), Utc::now())
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    // The consumed token survived the in-transaction purge (SQL assert).
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(true));
    // The expired sibling was purged by the same rotation.
    assert_eq!(token_row_count(&pool, user_id, "stale").await?, 0);
    // The replacement is present.
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 1);
    Ok(())
}

// After the migration that drops the redundant manual index on `token`,
// the UNIQUE column constraint's implicit index must keep serving every
// token-based path: the rotation's consumption predicate must resolve, and
// the query planner must answer a `WHERE token = ?` predicate with an index
// search (the implicit unique index), never a full table scan.
#[tokio::test]
async fn test_token_paths_resolve_and_stay_indexed_after_duplicate_index_drop() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    let now: DateTime<Utc> = Utc::now() + Duration::hours(1);
    let refresh_token = RefreshToken {
        id: 0,
        user_id,
        token: "token".to_string(),
        expire_at: now,
        created_at: now,
        is_used: false,
    };
    repository.insert(&refresh_token).await?;
    // The rotation's token-based consumption resolves after the index drop.
    let outcome = repository
        .rotate(user_id, "token", &new_replacement(user_id, now), Utc::now())
        .await?;
    assert!(matches!(outcome, RotationOutcome::Rotated(_)));
    // The consumed row stays observable through the token predicate.
    assert_eq!(token_row_count(&pool, user_id, "token").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "token").await?, Some(true));
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

// Small builders shared by the rotation tests: a fresh replacement token
// expiring one hour from `base`.
fn now_plus_1h() -> DateTime<Utc> {
    Utc::now() + Duration::hours(1)
}

fn new_replacement(user_id: u32, expire_at: DateTime<Utc>) -> RefreshToken {
    RefreshToken {
        id: 0,
        user_id,
        token: "replacement".to_string(),
        expire_at,
        created_at: Utc::now(),
        is_used: false,
    }
}

// Session-domain tests. Sessions share the refresh_tokens table but are
// stored under `grpc-session:`-tagged digests; these tests pin the at-rest
// format, the validation/revocation semantics, and the strict domain
// scoping of every refresh sweep and session sweep.

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

// Seeds one session for `user_id`, expiring one hour in the future.
async fn seed_session(
    repository: &RefreshTokenRepository,
    user_id: u32,
    token: &str,
) -> Result<()> {
    repository
        .insert_session(user_id, token, Utc::now() + Duration::hours(1), Utc::now())
        .await?;
    Ok(())
}

// At-rest format: the session row carries the exact tagged digest, the
// plaintext appears nowhere, and the value is NOT the bare refresh-format
// digest (the domain marker is what keeps the two token classes apart).
#[tokio::test]
async fn test_insert_session_stores_tagged_digest() -> Result<()> {
    use sha2::{Digest, Sha256};

    // Independent digest computation for the at-rest assertion (sha2
    // directly, not the repository helper, so the format pin is not
    // circular).
    fn digest_hex(value: &str) -> String {
        Sha256::digest(value.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session-token").await?;
    let rows = sqlx::query("SELECT token FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&*pool)
        .await?;
    assert_eq!(rows.len(), 1);
    let stored: String = rows[0].try_get("token")?;
    assert_eq!(
        stored,
        format!("grpc-session:{}", digest_hex("session-token"))
    );
    assert_ne!(stored, digest_hex("session-token"));
    // The plaintext token matches no stored row anywhere in the table.
    let plaintext = sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE token = ?")
        .bind("session-token")
        .fetch_one(&*pool)
        .await?;
    assert_eq!(plaintext.try_get::<i64, _>("n")?, 0);
    Ok(())
}

// Non-consuming validation: a valid presentation returns the owning user
// repeatedly; unknown values and digest strings (tagged or bare) match
// nothing — a database thief presenting a stolen digest as a token finds
// no session.
#[tokio::test]
async fn test_session_user_id_lookup_valid_unknown_and_digest_rejected() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session-token").await?;
    let first = repository
        .session_user_id("session-token", Utc::now())
        .await?;
    let second = repository
        .session_user_id("session-token", Utc::now())
        .await?;
    assert_eq!(first, Some(user_id));
    assert_eq!(second, Some(user_id));
    // Unknown values match nothing.
    assert_eq!(repository.session_user_id("other", Utc::now()).await?, None);
    // Presenting the stored tagged digest — or its bare hex form — as a
    // raw token double-hashes it and matches nothing.
    let tagged = session_digest("session-token");
    assert_eq!(repository.session_user_id(&tagged, Utc::now()).await?, None);
    assert_eq!(
        repository
            .session_user_id(&token_digest("session-token"), Utc::now())
            .await?,
        None
    );
    Ok(())
}

// Strictly-unexpired predicate: after the clock passes the expiry the
// session stops validating while the row itself stays untouched.
#[tokio::test]
async fn test_expired_session_not_validated() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session-token").await?;
    assert_eq!(
        repository
            .session_user_id("session-token", Utc::now())
            .await?,
        Some(user_id)
    );
    expire_session(&pool, user_id, "session-token").await?;
    assert_eq!(
        repository
            .session_user_id("session-token", Utc::now())
            .await?,
        None
    );
    // The row is still there (expiry alone is not deletion).
    assert_eq!(session_row_count(&pool, user_id, "session-token").await?, 1);
    Ok(())
}

// Single revocation: the revoked session stops validating, its sibling
// keeps working, and revoking again (or an unknown value) stays a no-op
// success.
#[tokio::test]
async fn test_revoked_session_not_validated_and_idempotent() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session-token").await?;
    seed_session(&repository, user_id, "other-session").await?;
    repository.revoke_session("session-token").await?;
    assert_eq!(
        repository
            .session_user_id("session-token", Utc::now())
            .await?,
        None
    );
    assert_eq!(
        repository
            .session_user_id("other-session", Utc::now())
            .await?,
        Some(user_id)
    );
    repository.revoke_session("session-token").await?;
    repository.revoke_session("never-stored").await?;
    Ok(())
}

// User-wide revocation: every session of the user flips to revoked while
// the user's refresh rows keep their exact pre-revocation state; another
// user's sessions are out of reach.
#[tokio::test]
async fn test_revoke_all_sessions_scope_keeps_refresh_rows() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session-a").await?;
    seed_session(&repository, user_id, "session-b").await?;
    repository
        .insert(&RefreshToken {
            id: 0,
            user_id,
            token: "refresh".to_string(),
            expire_at: now_plus_1h(),
            created_at: Utc::now(),
            is_used: false,
        })
        .await?;
    // Revoking every session of a nonexistent user changes nothing.
    repository.revoke_all_sessions(2606).await?;
    assert_eq!(
        repository.session_user_id("session-a", Utc::now()).await?,
        Some(user_id)
    );
    // Logout-everywhere: both sessions revoked, refresh row untouched.
    repository.revoke_all_sessions(user_id).await?;
    assert_eq!(
        repository.session_user_id("session-a", Utc::now()).await?,
        None
    );
    assert_eq!(
        repository.session_user_id("session-b", Utc::now()).await?,
        None
    );
    assert_eq!(token_row_count(&pool, user_id, "refresh").await?, 1);
    assert_eq!(token_is_used(&pool, user_id, "refresh").await?, Some(false));
    Ok(())
}

// Session expiry cleanup: expired session rows are removed while every
// unexpired row — session AND refresh — survives.
#[tokio::test]
async fn test_delete_expired_sessions_leaves_refresh_rows_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "live-session").await?;
    seed_session(&repository, user_id, "dead-session").await?;
    expire_session(&pool, user_id, "dead-session").await?;
    repository
        .insert(&RefreshToken {
            id: 0,
            user_id,
            token: "live-refresh".to_string(),
            expire_at: now_plus_1h(),
            created_at: Utc::now(),
            is_used: false,
        })
        .await?;
    repository.delete_expired_sessions(Utc::now()).await?;
    assert_eq!(session_row_count(&pool, user_id, "dead-session").await?, 0);
    assert_eq!(session_row_count(&pool, user_id, "live-session").await?, 1);
    assert_eq!(
        repository
            .session_user_id("live-session", Utc::now())
            .await?,
        Some(user_id)
    );
    assert_eq!(token_row_count(&pool, user_id, "live-refresh").await?, 1);
    Ok(())
}

// The three refresh sweeps (add-path purge, user-wide deletion, global
// expiry cleanup) never delete a session row.
#[tokio::test]
async fn test_refresh_sweeps_leave_session_rows_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session").await?;
    // One expired and one live refresh row for the same user.
    repository
        .insert(&RefreshToken {
            id: 0,
            user_id,
            token: "expired-refresh".to_string(),
            expire_at: Utc::now() - Duration::hours(1),
            created_at: Utc::now(),
            is_used: false,
        })
        .await?;
    repository
        .insert(&RefreshToken {
            id: 0,
            user_id,
            token: "live-refresh".to_string(),
            expire_at: now_plus_1h(),
            created_at: Utc::now(),
            is_used: false,
        })
        .await?;
    // Add-path expiry purge (delete_expired_for_user).
    repository
        .delete_expired_for_user(user_id, Utc::now())
        .await?;
    assert_eq!(session_row_count(&pool, user_id, "session").await?, 1);
    assert_eq!(token_row_count(&pool, user_id, "expired-refresh").await?, 0);
    // Global expiry cleanup (delete_expired behind cleanup_expired).
    repository.delete_expired(Utc::now()).await?;
    assert_eq!(session_row_count(&pool, user_id, "session").await?, 1);
    // User-wide refresh deletion (delete_by_user_id).
    repository.delete_by_user_id(user_id).await?;
    assert_eq!(token_row_count(&pool, user_id, "live-refresh").await?, 0);
    // The session row survived every refresh sweep.
    assert_eq!(session_row_count(&pool, user_id, "session").await?, 1);
    assert_eq!(
        repository.session_user_id("session", Utc::now()).await?,
        Some(user_id)
    );
    Ok(())
}

// Rotation-transaction purges: the expired-only retention purge and the
// replay-driven reuse purge both delete refresh rows only; the session row
// of the same user stays valid through a full rotation + replay cycle.
#[tokio::test]
async fn test_rotation_purges_leave_session_rows_intact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = RefreshTokenRepository::new(pool.clone());
    let user_id = test_user(&connection).await?;
    seed_session(&repository, user_id, "session").await?;
    repository
        .insert(&RefreshToken {
            id: 0,
            user_id,
            token: "refresh".to_string(),
            expire_at: now_plus_1h(),
            created_at: Utc::now(),
            is_used: false,
        })
        .await?;
    // Legitimate rotation (in-transaction retention purge ran).
    let rotated = repository
        .rotate(
            user_id,
            "refresh",
            &new_replacement(user_id, now_plus_1h()),
            Utc::now(),
        )
        .await?;
    assert!(matches!(rotated, RotationOutcome::Rotated(_)));
    assert_eq!(session_row_count(&pool, user_id, "session").await?, 1);
    // Replay (reuse purge deletes every refresh row of the user).
    let replay = repository
        .rotate(
            user_id,
            "refresh",
            &new_replacement(user_id, now_plus_1h()),
            Utc::now(),
        )
        .await?;
    assert_eq!(replay, RotationOutcome::ReuseDetected);
    assert_eq!(token_row_count(&pool, user_id, "replacement").await?, 0);
    // The session row is still present and still validates.
    assert_eq!(session_row_count(&pool, user_id, "session").await?, 1);
    assert_eq!(
        repository.session_user_id("session", Utc::now()).await?,
        Some(user_id)
    );
    Ok(())
}
