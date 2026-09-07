use super::*;
use crate::{
    db::connection::Connection,
    db::primary::{models::SortOrder, service::Service},
    users::model::UserSortKey,
};
use anyhow::Result;

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
