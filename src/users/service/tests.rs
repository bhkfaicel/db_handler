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
    id = id + 1;
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
                i.to_string(),
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
