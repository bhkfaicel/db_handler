use super::*;
use crate::{
    db::connection::Connection,
    db::primary::{models::SortOrder, service::Service},
    refresh_token::{error::RotationOutcome, service::RefreshTokenService},
    users::model::UserSortKey,
};
use anyhow::Result;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;

// Independent session-digest computation (sha2 directly, not a repository
// helper): at rest a session row stores `grpc-session:<sha256(raw)>`, and
// raw-count assertions from this test module must not depend on the
// private refresh-token internals — the same non-circular technique the
// refresh-token tests use.
fn session_digest_of(raw: &str) -> String {
    let hex: String = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("grpc-session:{hex}")
}

#[tokio::test]
async fn test_add_user() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "user name".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    assert!(id > 0);
    Ok(())
}

#[tokio::test]
async fn test_get_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let retrieved_user = service.get_by_id(id as u32).await?;
    assert!(retrieved_user.is_some());
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "username".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    assert_eq!(retrieved_user.id, id);
    Ok(())
}

#[tokio::test]
async fn test_get_by_id_not_found() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let mut id = service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    id += 1;
    let retrieved_user = service.get_by_id(id as u32).await?;
    assert!(retrieved_user.is_none());
    Ok(())
}

#[tokio::test]
async fn test_find_by_string() -> Result<()> {
    let connection = Connection::new().await?;
    let mut users: Vec<User> = Vec::new();
    let service = UserService::new(connection.clone());
    for i in 1..5 {
        users.push(User {
            id: i,
            username: format!("username{}", i),
            email: format!("faycelbhk{}@gmail.com", i),
        });
        service
            .add(
                format!("username{}", i),
                format!("faycelbhk{}@gmail.com", i),
                // Passwords must respect the 8..=128-byte house policy, so
                // the seeded fixture cannot use the raw index as password.
                format!("password{}", i),
            )
            .await?;
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: -1,
        sort_key: UserSortKey::Id,
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let results = service.find_by_string(&pagination_params).await?;
    assert_eq!(results.total, 4);
    for (i, user) in users.iter().enumerate() {
        let user_item = &results.items[i];
        assert_eq!(user.id, user_item.id);
        assert_eq!(user.username, user_item.username);
        assert_eq!(user.email, user_item.email);
    }
    Ok(())
}

#[tokio::test]
async fn test_delete_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    service.delete_by_id(id as u32).await?;
    let retrieved_user = service.get_by_id(id as u32).await?;
    assert!(retrieved_user.is_none());
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let retrieved_user = service
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(retrieved_user.is_some());
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "username".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    assert_eq!(retrieved_user.id, id);
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password_wrong_password() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let retrieved_user = service
        .get_by_email_and_password(
            "faycelbhk@gmail.com".to_string(),
            "wrong password".to_string(),
        )
        .await?;
    assert!(retrieved_user.is_none());
    Ok(())
}

#[tokio::test]
async fn update_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let updated_users_result = service
        .update_by_id(
            id,
            Some("username".to_string()),
            Some("email".to_string()),
            Some("123456789".to_string()),
        )
        .await?;
    assert_eq!(updated_users_result, 1);
    let retrieved_user = service.get_by_id(id as u32).await?;
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, id);
    assert_eq!(retrieved_user.username, "username".to_string());
    assert_eq!(retrieved_user.email, "email".to_string());
    let verif_password = service
        .get_by_email_and_password("email".to_string(), "123456789".to_string())
        .await?;
    assert!(verif_password.is_some());
    Ok(())
}

#[tokio::test]
async fn update_by_id_user_does_not_exits() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let updated_users_result = service
        .update_by_id(
            id + 1,
            Some("username".to_string()),
            Some("email".to_string()),
            Some("123456789".to_string()),
        )
        .await?;
    assert_eq!(updated_users_result, 0);
    let retrieved_user = service.get_by_id(id as u32).await?;
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, id);
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    let verif_password = service
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(verif_password.is_some());
    Ok(())
}

#[tokio::test]
async fn update_by_id_no_fields() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let updated_users_result = service.update_by_id(id, None, None, None).await;
    assert!(updated_users_result.is_err());
    let retrieved_user = service.get_by_id(id as u32).await?;
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, id);
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    let verif_password = service
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(verif_password.is_some());
    Ok(())
}

/// Typed non-regression test: adding a second user with an already
/// registered email must yield the dedicated `EmailAlreadyExists` variant
/// carrying the offending email, mapped from the structured unique-violation
/// constraint (`users.email`) reported by the repository.
#[tokio::test]
async fn add_duplicate_email_yields_email_already_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let duplicate = service
        .add(
            "another username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await;
    assert!(
        matches!(
            &duplicate,
            Err(UserError::EmailAlreadyExists(email)) if email == "faycelbhk@gmail.com"
        ),
        "expected UserError::EmailAlreadyExists, got: {duplicate:?}"
    );
    Ok(())
}

// Creation boundary policy: 7 and 129-byte passwords are rejected with the
// typed input error BEFORE any hashing work; the 8 and 128-byte bounds are
// accepted.
#[tokio::test]
async fn test_add_password_length_boundaries() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let too_short = service
        .add(
            "username".to_string(),
            "short@bhk.com".to_string(),
            "a".repeat(7),
        )
        .await;
    let too_long = service
        .add(
            "username".to_string(),
            "long@bhk.com".to_string(),
            "a".repeat(129),
        )
        .await;
    assert!(
        matches!(&too_short, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 7 bytes, got: {too_short:?}"
    );
    assert!(
        matches!(&too_long, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 129 bytes, got: {too_long:?}"
    );
    // Boundary accepts: exactly 8 and exactly 128 bytes create normally.
    let id_min = service
        .add("min".to_string(), "min@bhk.com".to_string(), "a".repeat(8))
        .await?;
    let id_max = service
        .add(
            "max".to_string(),
            "max@bhk.com".to_string(),
            "a".repeat(128),
        )
        .await?;
    assert!(id_min > 0);
    assert!(id_max > 0);
    Ok(())
}

// Update (reset) boundary policy: same rejection before hashing for
// out-of-range reset passwords, acceptance at the 8 and 128-byte bounds.
#[tokio::test]
async fn test_update_by_id_password_length_boundaries() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "reset@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let too_short = service
        .update_by_id(id, None, None, Some("a".repeat(7)))
        .await;
    let too_long = service
        .update_by_id(id, None, None, Some("a".repeat(129)))
        .await;
    assert!(
        matches!(&too_short, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 7 bytes, got: {too_short:?}"
    );
    assert!(
        matches!(&too_long, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 129 bytes, got: {too_long:?}"
    );
    // A rejected reset must not have changed anything: the original
    // password still authenticates.
    assert!(service
        .get_by_email_and_password("reset@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    // Boundary accepts: reset to exactly 8 then exactly 128 bytes works and
    // the new password authenticates.
    let reset_min = service
        .update_by_id(id, None, None, Some("a".repeat(8)))
        .await?;
    assert_eq!(reset_min, 1);
    assert!(service
        .get_by_email_and_password("reset@bhk.com".to_string(), "a".repeat(8))
        .await?
        .is_some());
    let reset_max = service
        .update_by_id(id, None, None, Some("a".repeat(128)))
        .await?;
    assert_eq!(reset_max, 1);
    assert!(service
        .get_by_email_and_password("reset@bhk.com".to_string(), "a".repeat(128))
        .await?
        .is_some());
    Ok(())
}

// Uniform login rejection at the service level: an out-of-range password
// (7 or 129 bytes) returns `None` whether the email exists or not, and
// without any Argon2 work (the repository gates before its SQL lookup), so
// an invalid length can never probe account existence.
#[tokio::test]
async fn test_get_by_email_and_password_out_of_range_password_uniform_none() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "uniform@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    // Present email, too-short password.
    let present_short = service
        .get_by_email_and_password("uniform@bhk.com".to_string(), "a".repeat(7))
        .await?;
    // Absent email, too-short password.
    let absent_short = service
        .get_by_email_and_password("ghost@bhk.com".to_string(), "a".repeat(7))
        .await?;
    // Present email, too-long password (129 bytes).
    let present_long = service
        .get_by_email_and_password("uniform@bhk.com".to_string(), "a".repeat(129))
        .await?;
    // Absent email, too-long password.
    let absent_long = service
        .get_by_email_and_password("ghost@bhk.com".to_string(), "a".repeat(129))
        .await?;
    assert!(present_short.is_none());
    assert!(absent_short.is_none());
    assert!(present_long.is_none());
    assert!(absent_long.is_none());
    // The valid-length correct password still authenticates.
    assert!(service
        .get_by_email_and_password("uniform@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    Ok(())
}

// Exact-email lookup through the public API: the user is found by its
// byte-exact email and the returned model is the public one (id, username,
// email) — it structurally carries no password-hash field at all.
#[tokio::test]
async fn test_get_by_email_finds_user_without_hash_material() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "lookup@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let found = service
        .get_by_email("lookup@bhk.com".to_string())
        .await?
        .expect("exact email must resolve to the user");
    assert_eq!(
        found,
        User::new(id, "username".to_string(), "lookup@bhk.com".to_string())
    );
    Ok(())
}

// An email that matches no user reports `Ok(None)` — a lookup, not an
// error (the typed not-found error is reserved for the reset path).
#[tokio::test]
async fn test_get_by_email_unknown_returns_none() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    assert!(service
        .get_by_email("ghost@bhk.com".to_string())
        .await?
        .is_none());
    Ok(())
}

// Email lookups stay BYTE-EXACT on the plain lookup too: case and
// whitespace variants are different emails, only the exact form matches.
#[tokio::test]
async fn test_get_by_email_is_byte_exact() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "Mixed@Example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    // Lowercased, uppercased, trailing and leading whitespace variants:
    // each is a different email, no user.
    for variant in [
        "mixed@example.com",
        "MIXED@EXAMPLE.COM",
        "Mixed@Example.com ",
        " Mixed@Example.com",
    ] {
        assert!(
            service.get_by_email(variant.to_string()).await?.is_none(),
            "variant {variant:?} must not match the stored email"
        );
    }
    // The byte-exact form matches.
    assert!(service
        .get_by_email("Mixed@Example.com".to_string())
        .await?
        .is_some());
    Ok(())
}

// Reset policy gate: 7- and 129-byte passwords are rejected with the typed
// input error BEFORE any hashing or SQL runs — the old password still
// authenticates and every live credential of the user survives untouched.
#[tokio::test]
async fn test_reset_password_rejects_out_of_range_passwords_before_sql() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let user_id = service
        .add(
            "username".to_string(),
            "bounds@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens
        .add(user_id, "refresh-raw", future, Utc::now())
        .await?;
    tokens
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;

    let too_short = service
        .reset_password_revoking_tokens("bounds@bhk.com".to_string(), "a".repeat(7))
        .await;
    let too_long = service
        .reset_password_revoking_tokens("bounds@bhk.com".to_string(), "a".repeat(129))
        .await;
    assert!(
        matches!(&too_short, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 7 bytes, got: {too_short:?}"
    );
    assert!(
        matches!(&too_long, Err(UserError::InvalidInput(_))),
        "expected UserError::InvalidInput for 129 bytes, got: {too_long:?}"
    );

    // No side effect at all: the old password authenticates and both live
    // credentials still operate.
    assert!(service
        .get_by_email_and_password("bounds@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    assert!(matches!(
        tokens
            .rotate(user_id, "refresh-raw", "x", future, Utc::now())
            .await?,
        RotationOutcome::Rotated(_)
    ));
    assert_eq!(tokens.validate_session("session-raw").await?, Some(user_id));
    Ok(())
}

// Unknown reset email: the dedicated typed not-found error, and no side
// effect anywhere — an existing user's password and live credentials are
// untouched (nothing is written for an email that matches no user).
#[tokio::test]
async fn test_reset_password_unknown_email_typed_error_without_side_effects() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let user_id = service
        .add(
            "username".to_string(),
            "present@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens
        .add(user_id, "refresh-raw", future, Utc::now())
        .await?;
    tokens
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;

    let result = service
        .reset_password_revoking_tokens("ghost@bhk.com".to_string(), "new password".to_string())
        .await;
    assert!(
        matches!(
            &result,
            Err(UserError::NotFoundByEmail(email)) if email == "ghost@bhk.com"
        ),
        "expected UserError::NotFoundByEmail, got: {result:?}"
    );

    // The existing user kept everything.
    assert!(service
        .get_by_email_and_password("present@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    assert!(matches!(
        tokens
            .rotate(user_id, "refresh-raw", "x", future, Utc::now())
            .await?,
        RotationOutcome::Rotated(_)
    ));
    assert_eq!(tokens.validate_session("session-raw").await?, Some(user_id));
    Ok(())
}

// Success path: the password change and the deletion of EVERY credential
// row of the user — REST refresh rows AND session rows — commit together,
// and stay strictly scoped to the target user (another user's credentials
// keep working).
#[tokio::test]
async fn test_reset_password_revokes_both_domains_with_user_isolation() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = UserService::new(connection.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let user_a = service
        .add(
            "alice".to_string(),
            "alice@bhk.com".to_string(),
            "old password".to_string(),
        )
        .await?;
    let user_b = service
        .add(
            "bob".to_string(),
            "bob@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens.add(user_a, "a-refresh", future, Utc::now()).await?;
    tokens
        .add_session(user_a, "a-session", future, Utc::now())
        .await?;
    tokens.add(user_b, "b-refresh", future, Utc::now()).await?;
    tokens
        .add_session(user_b, "b-session", future, Utc::now())
        .await?;

    let reset_user = service
        .reset_password_revoking_tokens("alice@bhk.com".to_string(), "new password".to_string())
        .await?;
    assert_eq!(
        reset_user,
        User::new(user_a, "alice".to_string(), "alice@bhk.com".to_string())
    );

    // Alice: the old password fails, the new one passes.
    assert!(service
        .get_by_email_and_password("alice@bhk.com".to_string(), "old password".to_string())
        .await?
        .is_none());
    assert!(service
        .get_by_email_and_password("alice@bhk.com".to_string(), "new password".to_string())
        .await?
        .is_some());
    // Alice: the refresh row is GONE — rotation classifies NotFound (a
    // merely revoked row would still exist and classify as reuse).
    let a_refresh = tokens
        .rotate(user_a, "a-refresh", "x", future, Utc::now())
        .await?;
    assert_eq!(a_refresh, RotationOutcome::NotFound);
    // Alice: the session row is GONE from storage, not just revoked (raw
    // count through the domain-tagged digest boundary the storage uses).
    let a_session_rows =
        sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ? AND token = ?")
            .bind(user_a)
            .bind(session_digest_of("a-session"))
            .fetch_one(&*pool)
            .await?;
    assert_eq!(a_session_rows.try_get::<i64, _>("n")?, 0);
    assert_eq!(tokens.validate_session("a-session").await?, None);

    // Bob: strictly untouched in both domains.
    assert!(service
        .get_by_email_and_password("bob@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    assert!(matches!(
        tokens
            .rotate(user_b, "b-refresh", "x", future, Utc::now())
            .await?,
        RotationOutcome::Rotated(_)
    ));
    assert_eq!(tokens.validate_session("b-session").await?, Some(user_b));
    Ok(())
}

// The SAME raw value stored in both domains for one user is invalidated in
// BOTH domains by one reset (and it took two distinct rows to store it: the
// domain-separation storage identity, unaffected by the reset path).
#[tokio::test]
async fn test_reset_password_invalidates_same_raw_token_in_both_domains() -> Result<()> {
    let connection = Connection::new().await?;
    let service = UserService::new(connection.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let user_id = service
        .add(
            "username".to_string(),
            "dual@bhk.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens.add(user_id, "same-raw", future, Utc::now()).await?;
    tokens
        .add_session(user_id, "same-raw", future, Utc::now())
        .await?;

    service
        .reset_password_revoking_tokens("dual@bhk.com".to_string(), "new password".to_string())
        .await?;

    // Both domains report the credential unusable.
    let refresh = tokens
        .rotate(user_id, "same-raw", "x", future, Utc::now())
        .await?;
    assert_eq!(refresh, RotationOutcome::NotFound);
    assert_eq!(tokens.validate_session("same-raw").await?, None);
    Ok(())
}

// Atomic rollback under database-enforced failure injection: a test-only
// SQLite trigger aborts any deletion from refresh_tokens for the target
// user, so the reset's cross-domain invalidation fails AFTER the password
// UPDATE ran inside the open transaction. The error must propagate and the
// whole transaction roll back: old password still authenticates, the new
// one does not, and both pre-existing live credentials still operate. The
// trigger is removed afterwards (asserted gone), leaving no test residue.
#[tokio::test]
async fn test_reset_password_rolls_back_when_token_invalidation_fails() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let service = UserService::new(connection.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let user_id = service
        .add(
            "username".to_string(),
            "rollback@bhk.com".to_string(),
            "old password".to_string(),
        )
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens
        .add(user_id, "refresh-raw", future, Utc::now())
        .await?;
    tokens
        .add_session(user_id, "session-raw", future, Utc::now())
        .await?;

    // Test-only failure injection: SQLite ABORTs any deletion from
    // refresh_tokens belonging to the target user. The user id comes from
    // this test's own fixture (never caller input), so inlining it into
    // the trigger definition is safe; parameters are not allowed inside
    // trigger bodies anyway.
    sqlx::query(&format!(
        "CREATE TRIGGER reset_rollback_injector \
         BEFORE DELETE ON refresh_tokens \
         WHEN OLD.user_id = {user_id} \
         BEGIN SELECT RAISE(ABORT, 'injected refresh_tokens delete failure'); END;"
    ))
    .execute(&*pool)
    .await?;

    let result = service
        .reset_password_revoking_tokens("rollback@bhk.com".to_string(), "new password".to_string())
        .await;
    assert!(
        matches!(&result, Err(UserError::Common(CommonError::Database(_)))),
        "the injected invalidation failure must propagate as a database error, got: {result:?}"
    );

    // Remove the trigger BEFORE probing the post-rollback world.
    sqlx::query("DROP TRIGGER reset_rollback_injector")
        .execute(&*pool)
        .await?;
    let triggers_left = sqlx::query(
        "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'trigger' AND name = 'reset_rollback_injector'",
    )
    .fetch_one(&*pool)
    .await?;
    assert_eq!(triggers_left.try_get::<i64, _>("n")?, 0);

    // The password UPDATE was rolled back: old password authenticates,
    // the new one does not.
    assert!(service
        .get_by_email_and_password("rollback@bhk.com".to_string(), "old password".to_string())
        .await?
        .is_some());
    assert!(service
        .get_by_email_and_password("rollback@bhk.com".to_string(), "new password".to_string())
        .await?
        .is_none());
    // Both pre-existing live credentials still operate: the refresh token
    // rotates (proof the row was never deleted) and the session validates.
    assert!(matches!(
        tokens
            .rotate(user_id, "refresh-raw", "rotated", future, Utc::now())
            .await?,
        RotationOutcome::Rotated(_)
    ));
    assert_eq!(tokens.validate_session("session-raw").await?, Some(user_id));
    Ok(())
}
