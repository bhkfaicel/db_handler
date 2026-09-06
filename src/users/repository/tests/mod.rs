use super::*;
use crate::{
    db::connection::Connection,
    db::primary::{error::CommonError, models::SortOrder, service::Service},
    users::{model::User, service::UserService},
};
// The repository persists the hash it is handed, so the update fixtures
// need a real bcrypt hash; build it with bcrypt directly instead of
// reaching into the users-domain hashing helper.
use bcrypt::{hash, DEFAULT_COST};
// Explicit anyhow Result: the repository module now re-exports the
// `CommonError`-based `Result` alias, which must not shadow the tests'
// return type.
use anyhow::{Ok, Result};

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
            Some(hash("123456789", DEFAULT_COST).unwrap()),
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
            // Same contract as above: hand the repository a bcrypt hash.
            Some(hash("123456789", DEFAULT_COST).unwrap()),
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
            Some(hash("123456789", DEFAULT_COST).unwrap()),
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
