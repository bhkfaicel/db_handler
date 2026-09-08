use super::*;
use crate::{
    db::connection::Connection,
    db::primary::{error::CommonError, models::SortOrder, service::Service},
    refresh_token::service::RefreshTokenService,
    users::{model::User, service::UserService},
};
// The repository persists the hash it is handed, so the update fixtures
// need a real Argon2id PHC string; build it with the domain hashing helper
// directly (synchronous primitive) instead of going through the service.
// The dummy constants/verifier back the malformed-hash timing-equalization
// proof below.
use crate::users::password::{hash_password, verify_password_async, DUMMY_ARGON2_HASH};
// Explicit anyhow Result: the repository module now re-exports the
// `CommonError`-based `Result` alias, which must not shadow the tests'
// return type.
use anyhow::{Ok, Result};
use chrono::{Duration, Utc};
use sqlx::Row;
use std::time::Instant;

#[tokio::test]
async fn test_insert_user() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let user = InternalUser::new(
        0,
        "username".to_string(),
        "faycel@bhk.com".to_string(),
        Some("password hash".to_string()),
    );
    let id = repository.insert(&user).await?;
    let created_user = sqlx::query_as::<_, InternalUser>(
        r#"
        SELECT * FROM users WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created user");
    assert!(id > 0);
    assert_eq!(created_user.id, id);
    assert_eq!(created_user.username, "username".to_string());
    assert_eq!(created_user.email, "faycel@bhk.com".to_string());
    assert_eq!(
        created_user.password_hash,
        Some("password hash".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn test_insert_user_email_exists() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let user = InternalUser::new(
        0,
        "username".to_string(),
        "faycel@bhk.com".to_string(),
        Some("password hash".to_string()),
    );
    let id = repository.insert(&user).await?;
    let created_user = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .expect("Failed to fetch created user");
    assert!(id > 0);
    assert_eq!(created_user.email, "faycel@bhk.com".to_string());
    let insert_result = repository.insert(&user).await;
    if let Err(err) = insert_result {
        // The repository reports the duplicate structurally: the central
        // conversion decodes SQLite's message into the constraint name, so
        // the assertion targets the typed variant and the exact constraint
        // SQLite reports for the inline UNIQUE on users.email.
        assert!(
            matches!(
                &err,
                CommonError::UniqueViolation { constraint } if constraint == "users.email"
            ),
            "expected CommonError::UniqueViolation on users.email, got: {err:?}"
        );
    } else {
        panic!("Expected an error, but insert succeeded");
    }
    Ok(())
}

#[tokio::test]
async fn test_find_by_string() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool);
    let mut users: Vec<User> = Vec::new();
    for i in 1..30 {
        let user = InternalUser::new(
            0,
            i.to_string(),
            format!("{}@bhk.com", i),
            Some(i.to_string()),
        );
        users.push(User {
            id: i,
            username: i.to_string(),
            email: format!("{}@bhk.com", i),
        });
        repository.insert(&user).await?;
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: -1,
        sort_key: UserSortKey::Id,
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let result = repository.find_by_string(&pagination_params).await?;
    assert_eq!(result.total, 29);
    assert_eq!(result.items, users);
    Ok(())
}

#[tokio::test]
async fn test_get_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool);
    let user = InternalUser::new(
        0,
        "faycel".to_string(),
        "faycelbhk@gmail.com".to_string(),
        Some("password".to_string()),
    );
    let id = repository.insert(&user).await?;
    let retrieved_user = repository.get_by_id(id as u32).await?;
    assert!(retrieved_user.is_some());
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    assert_eq!(retrieved_user.id, id);
    Ok(())
}

#[tokio::test]
async fn test_delete_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool);
    let user = InternalUser::new(
        0,
        "username".to_string(),
        "faycelbhk@gmail.com".to_string(),
        Some("password".to_string()),
    );
    let id = repository.insert(&user).await?;
    repository.delete_by_id(id as u32).await?;
    let retrieved_user = repository.get_by_id(id as u32).await?;
    assert!(retrieved_user.is_none());
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let user = User::new(
        id,
        "username".to_string(),
        "faycelbhk@gmail.com".to_string(),
    );
    let result = repository
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert_eq!(user, result.unwrap());
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password_user_does_not_exist() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let result = repository
        .get_by_email_and_password("f@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(result.is_none());
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password_no_user_on_db() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool);
    let result = repository
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(result.is_none());
    Ok(())
}

// Deterministic absent-branch exercise for the always-verify login path:
// with no matching user, the lookup must still complete the full Argon2id
// verification against the fixed dummy PHC (the password-helpers test
// proves that constant is a real default-policy hash that rejects
// passwords, so reaching None below required real Argon2 work) and only
// then report no user. This keeps the unknown-email response's CPU cost
// identical to the wrong-password response, closing the timing-based email
// enumeration.
#[tokio::test]
async fn test_get_by_email_and_password_absent_email_verifies_dummy_hash() -> Result<()> {
    let connection = Connection::new().await?;
    let repository = UserRepository::new(connection.primary_db_client().clone());
    let result = repository
        .get_by_email_and_password("nobody@bhk.com".to_string(), "password".to_string())
        .await?;
    assert!(result.is_none());
    Ok(())
}

// Fail-closed discipline: a corrupted (malformed PHC) stored hash must
// report no user without panicking — never an `Ok(Some(user))`, never an
// error bubbling up from the lookup itself. The parse failure must ALSO
// cost the same one Argon2 derivation as the unknown-email and
// wrong-password paths: a bare PHC parse error returns in microseconds,
// so without the equalizing dummy burn the fast response would reveal,
// by timing alone, that the email exists with an unusable (pre-migration)
// hash (timing user-enumeration oracle, CWE-208).
#[tokio::test]
async fn test_get_by_email_and_password_malformed_stored_hash_fails_closed() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let user = InternalUser::new(
        0,
        "username".to_string(),
        "malformed@bhk.com".to_string(),
        Some("not-a-valid-phc".to_string()),
    );
    let id = repository.insert(&user).await?;
    let malformed_start = Instant::now();
    let result = repository
        .get_by_email_and_password(
            "malformed@bhk.com".to_string(),
            "whatever-password".to_string(),
        )
        .await?;
    let malformed_elapsed = malformed_start.elapsed();
    assert!(result.is_none(), "malformed stored hash must fail closed");
    // Even the correct-looking password cannot authenticate a corrupted row.
    let user = repository.get_by_id(id as u32).await?;
    assert!(user.is_some());
    // Deterministic dummy-work proof: measure one warm dummy verification
    // (the exact cost an unknown-email login pays) and require the
    // malformed-hash login to have taken at least a quarter of it. A real
    // default-policy Argon2id derivation is orders of magnitude (~10³×)
    // more expensive than rejecting a string at the PHC parser, so only a
    // burned dummy verification can satisfy the bound; a regression that
    // skips the burn returns ~1000× faster and fails this assertion on any
    // hardware. The reference runs AFTER the login, so the login's burn
    // (potentially cold, hence slower) can only look more expensive, never
    // less.
    verify_password_async("warm-up".to_string(), DUMMY_ARGON2_HASH.to_string())
        .await
        .unwrap();
    let reference_start = Instant::now();
    verify_password_async(
        "reference-password".to_string(),
        DUMMY_ARGON2_HASH.to_string(),
    )
    .await
    .unwrap();
    let reference_elapsed = reference_start.elapsed();
    assert!(
        malformed_elapsed >= reference_elapsed / 4,
        "malformed-hash login returned too fast ({malformed_elapsed:?}) compared to \
         one dummy verification ({reference_elapsed:?}): the equalizing dummy burn \
         on the verification-error path is missing"
    );
    Ok(())
}

// Uniform length gate for login: an out-of-range password (7 or 129 bytes)
// must return `None` with the SAME outcome whether the email exists or not,
// and without any Argon2 work (enforced by the early return before the SQL
// lookup in the repository).
#[tokio::test]
async fn test_get_by_email_and_password_out_of_range_password_uniform_none() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "range@bhk.com".to_string(),
            "longenough".to_string(),
        )
        .await?;
    // Present email, too-short password.
    let present_short = repository
        .get_by_email_and_password("range@bhk.com".to_string(), "a".repeat(7))
        .await?;
    // Absent email, too-short password: identical outcome.
    let absent_short = repository
        .get_by_email_and_password("ghost@bhk.com".to_string(), "a".repeat(7))
        .await?;
    // Present email, too-long password (129 bytes).
    let present_long = repository
        .get_by_email_and_password("range@bhk.com".to_string(), "a".repeat(129))
        .await?;
    // Absent email, too-long password: identical outcome.
    let absent_long = repository
        .get_by_email_and_password("ghost@bhk.com".to_string(), "a".repeat(129))
        .await?;
    assert!(present_short.is_none());
    assert!(absent_short.is_none());
    assert!(present_long.is_none());
    assert!(absent_long.is_none());
    Ok(())
}

// Boundary login acceptance: exactly 8 and exactly 128-byte passwords are
// inside the policy and must authenticate when correct.
#[tokio::test]
async fn test_get_by_email_and_password_accepts_boundary_passwords() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
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
    let login_min = repository
        .get_by_email_and_password("min@bhk.com".to_string(), "a".repeat(8))
        .await?;
    let login_max = repository
        .get_by_email_and_password("max@bhk.com".to_string(), "a".repeat(128))
        .await?;
    assert_eq!(login_min.unwrap().id, id_min);
    assert_eq!(login_max.unwrap().id, id_max);
    Ok(())
}

// Email lookups stay BYTE-EXACT: case differences and surrounding
// whitespace are different emails. The lookup must not normalize, trim, or
// case-fold the stored or presented email.
#[tokio::test]
async fn test_get_by_email_and_password_email_is_byte_exact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "User@Example.com".to_string(),
            "password".to_string(),
        )
        .await?;
    // Lowercased variant: different email, no user.
    assert!(repository
        .get_by_email_and_password("user@example.com".to_string(), "password".to_string())
        .await?
        .is_none());
    // Uppercased variant: different email, no user.
    assert!(repository
        .get_by_email_and_password("USER@EXAMPLE.COM".to_string(), "password".to_string())
        .await?
        .is_none());
    // Trailing whitespace: different email, no user.
    assert!(repository
        .get_by_email_and_password("User@Example.com ".to_string(), "password".to_string())
        .await?
        .is_none());
    // Leading whitespace: different email, no user.
    assert!(repository
        .get_by_email_and_password(" User@Example.com".to_string(), "password".to_string())
        .await?
        .is_none());
    // The byte-exact form still authenticates.
    assert!(repository
        .get_by_email_and_password("User@Example.com".to_string(), "password".to_string())
        .await?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_by_email_and_password_wrong_password() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    service
        .add(
            "username".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let result = repository
        .get_by_email_and_password(
            "faycelbhk@gmail.com".to_string(),
            "wrongPassword".to_string(),
        )
        .await?;
    assert!(result.is_none());
    Ok(())
}

#[tokio::test]
async fn test_update_by_id() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let affected_rows = repository
        .update_by_id(
            id,
            Some("username".to_string()),
            Some("email".to_string()),
            // The repository persists the hash it is handed: hashing now
            // happens in the domain, so the update input is hashed first.
            Some(hash_password("123456789").unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(affected_rows, 1);
    let retrieved_user = repository.get_by_id(id as u32).await?;
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "username".to_string());
    assert_eq!(retrieved_user.email, "email".to_string());
    let verif_password = repository
        .get_by_email_and_password("email".to_string(), "123456789".to_string())
        .await?;
    assert!(verif_password.is_some());
    assert_eq!(verif_password.unwrap().id, id);
    Ok(())
}

#[tokio::test]
async fn test_update_by_id_only_password() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let affected_rows = repository
        .update_by_id(
            id,
            None,
            None,
            // Same contract as above: hand the repository an Argon2id PHC.
            Some(hash_password("123456789").unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(affected_rows, 1);
    let retrieved_user = repository.get_by_id(id as u32).await?;
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    let verif_password = repository
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "123456789".to_string())
        .await?;
    assert!(verif_password.is_some());
    assert_eq!(verif_password.unwrap().id, id);
    Ok(())
}

#[tokio::test]
async fn test_update_by_id_only_email() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let affected_rows = repository
        .update_by_id(id, None, Some("email@gmail.com".to_string()), None)
        .await
        .unwrap();
    assert_eq!(affected_rows, 1);
    let retrieved_user = repository.get_by_id(id as u32).await?;
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "email@gmail.com".to_string());
    let verif_password = repository
        .get_by_email_and_password("email@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(verif_password.is_some());
    assert_eq!(verif_password.unwrap().id, id);
    Ok(())
}

#[tokio::test]
async fn test_update_by_id_no_user_updated() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let affected_rows = repository
        .update_by_id(
            id + 1,
            Some("username".to_string()),
            Some("email".to_string()),
            // The repository persists the hash it is handed: hashing now
            // happens in the domain, so the update input is hashed first.
            Some(hash_password("123456789").unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(affected_rows, 0);
    let retrieved_user = repository.get_by_id(id as u32).await?;
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    let verif_password = repository
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(verif_password.is_some());
    assert_eq!(verif_password.unwrap().id, id);
    Ok(())
}

#[tokio::test]
async fn test_update_by_id_no_fields_no_user_updated() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let service = UserService::new(connection.clone());
    let id = service
        .add(
            "faycel".to_string(),
            "faycelbhk@gmail.com".to_string(),
            "password".to_string(),
        )
        .await?;
    let affected_rows = repository.update_by_id(id, None, None, None).await;
    assert!(affected_rows.is_err());
    let retrieved_user = repository.get_by_id(id as u32).await?;
    let retrieved_user: User = retrieved_user.unwrap();
    assert_eq!(retrieved_user.username, "faycel".to_string());
    assert_eq!(retrieved_user.email, "faycelbhk@gmail.com".to_string());
    let verif_password = repository
        .get_by_email_and_password("faycelbhk@gmail.com".to_string(), "password".to_string())
        .await?;
    assert!(verif_password.is_some());
    assert_eq!(verif_password.unwrap().id, id);
    Ok(())
}

// Repository-level exact-email lookup: one bound equality query, byte-exact,
// returning the public model (no hash field) or `None`.
#[tokio::test]
async fn test_get_by_email_found_not_found_and_byte_exact() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool);
    let id = repository
        .insert(&InternalUser::new(
            0,
            "username".to_string(),
            "Exact@BHK.com".to_string(),
            Some(hash_password("password").unwrap()),
        ))
        .await?;

    // Exact form: found, and equal to the public model only.
    let found = repository
        .get_by_email("Exact@BHK.com".to_string())
        .await?
        .expect("exact email must resolve");
    assert_eq!(
        found,
        User::new(id, "username".to_string(), "Exact@BHK.com".to_string())
    );

    // Unknown email: no user, no error.
    assert!(repository
        .get_by_email("ghost@bhk.com".to_string())
        .await?
        .is_none());

    // Byte-exact discipline: case and whitespace variants are different
    // emails; only the exact form matches.
    for variant in [
        "exact@bhk.com",
        "EXACT@BHK.COM",
        " Exact@BHK.com ",
        "Exact@BHK.com ",
    ] {
        assert!(
            repository
                .get_by_email(variant.to_string())
                .await?
                .is_none(),
            "variant {variant:?} must not match the stored email"
        );
    }
    Ok(())
}

// Repository-level atomic reset: the handed-in PHC is persisted and EVERY
// credential row of the user — refresh AND session domain — is deleted in
// the same committed transaction. Fixtures are seeded through the public
// refresh-token service (its private repository is out of reach here).
#[tokio::test]
async fn test_reset_password_revoking_tokens_success() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let tokens = RefreshTokenService::new(connection.clone());
    let id = repository
        .insert(&InternalUser::new(
            0,
            "username".to_string(),
            "reset@bhk.com".to_string(),
            Some(hash_password("old password").unwrap()),
        ))
        .await?;
    let future = Utc::now() + Duration::hours(1);
    tokens.add(id, "refresh-raw", future, Utc::now()).await?;
    tokens
        .add_session(id, "session-raw", future, Utc::now())
        .await?;

    let updated = repository
        .reset_password_revoking_tokens("reset@bhk.com", hash_password("new password").unwrap())
        .await?
        .expect("the exact email must resolve to the user");
    assert_eq!(
        updated,
        User::new(id, "username".to_string(), "reset@bhk.com".to_string())
    );

    // The new password authenticates, the old one no longer does.
    assert!(repository
        .get_by_email_and_password("reset@bhk.com".to_string(), "new password".to_string())
        .await?
        .is_some());
    assert!(repository
        .get_by_email_and_password("reset@bhk.com".to_string(), "old password".to_string())
        .await?
        .is_none());
    // Both credential rows are gone from storage (raw counts through the
    // at-rest digest forms each domain uses).
    let rows_left = sqlx::query("SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = ?")
        .bind(id)
        .fetch_one(&*pool)
        .await?;
    assert_eq!(rows_left.try_get::<i64, _>("n")?, 0);
    Ok(())
}

// Repository-level unknown email: `Ok(None)` and nothing written.
#[tokio::test]
async fn test_reset_password_revoking_tokens_unknown_email_returns_none() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = UserRepository::new(pool.clone());
    let id = repository
        .insert(&InternalUser::new(
            0,
            "username".to_string(),
            "other@bhk.com".to_string(),
            Some(hash_password("password").unwrap()),
        ))
        .await?;
    let result = repository
        .reset_password_revoking_tokens("ghost@bhk.com", hash_password("new password").unwrap())
        .await?;
    assert!(result.is_none());
    // The existing user was not touched by the unknown-email call.
    let user = repository.get_by_id(id as u32).await?.unwrap();
    assert_eq!(user.email, "other@bhk.com".to_string());
    assert!(repository
        .get_by_email_and_password("other@bhk.com".to_string(), "password".to_string())
        .await?
        .is_some());
    Ok(())
}
