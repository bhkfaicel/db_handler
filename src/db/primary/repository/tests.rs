use crate::{
    db::primary::{
        models::{PaginationParams, SortOrder},
        repository::get_paginated_results_by_string,
    },
    drives::columns::HasColumns,
};
use anyhow::Result;
use db_derive::HasColumns;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::time::Duration;
#[tokio::test]
async fn test_get_some_fields() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
        id: i64,
        username: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: Some("2@bhk".to_string()),
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, 3);
    let user1 = User {
        email: "faycel2@bhk.com".to_string(),
        id: 2,
        username: "2".to_string(),
    };
    assert_eq!(user1, result.items[0]);
    let user2 = User {
        email: "faycel12@bhk.com".to_string(),
        id: 12,
        username: "12".to_string(),
    };
    assert_eq!(user2, result.items[1]);
    Ok(())
}

#[tokio::test]
async fn test_get_sort_desc() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
        id: i64,
        username: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Desc,
        searched_value: Some("2@bhk".to_string()),
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, 3);
    let user1 = User {
        email: "faycel22@bhk.com".to_string(),
        id: 22,
        username: "22".to_string(),
    };
    assert_eq!(user1, result.items[0]);
    let user2 = User {
        email: "faycel12@bhk.com".to_string(),
        id: 12,
        username: "12".to_string(),
    };
    assert_eq!(user2, result.items[1]);
    Ok(())
}

#[tokio::test]
async fn test_get_with_skip() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
        id: i64,
        username: String,
    }
    let pagination_params = PaginationParams {
        skip: 1,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: Some("2@bhk".to_string()),
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, 3);
    let user1 = User {
        email: "faycel12@bhk.com".to_string(),
        id: 12,
        username: "12".to_string(),
    };
    assert_eq!(user1, result.items[0]);
    let user2 = User {
        email: "faycel22@bhk.com".to_string(),
        id: 22,
        username: "22".to_string(),
    };
    assert_eq!(user2, result.items[1]);
    Ok(())
}

#[tokio::test]
async fn test_get_one_field_diffrent_of_the_sort_key() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: Some("2@bhk".to_string()),
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, 3);
    let user1 = User {
        email: "faycel2@bhk.com".to_string(),
    };
    assert_eq!(user1, result.items[0]);
    let user2 = User {
        email: "faycel12@bhk.com".to_string(),
    };
    assert_eq!(user2, result.items[1]);
    Ok(())
}

#[tokio::test]
async fn test_get_searched_value_is_none() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
        id: i64,
        username: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, count_result);
    let user1 = User {
        email: "faycel1@bhk.com".to_string(),
        id: 1,
        username: "1".to_string(),
    };
    assert_eq!(user1, result.items[0]);
    let user2 = User {
        email: "faycel2@bhk.com".to_string(),
        id: 2,
        username: "2".to_string(),
    };
    assert_eq!(user2, result.items[1]);
    Ok(())
}

#[tokio::test]
async fn test_get_nbr_is_negative_and_searched_value_is_none() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    let mut users: Vec<User> = Vec::new();

    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
        users.push(User {
            email: format!("faycel{}@bhk.com", i),
            id: i,
            username: i.to_string(),
        });
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        email: String,
        id: i64,
        username: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: -1,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let result = get_paginated_results_by_string::<User>(&pool, table, &pagination_params).await?;
    assert_eq!(result.total, count_result);
    assert_eq!(result.items, users);
    Ok(())
}

#[tokio::test]
async fn test_one_field_with_type_not_string() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        id: i64,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: Some("2@bhk".to_string()),
    };
    assert!(
        get_paginated_results_by_string::<User>(&pool, table, &pagination_params)
            .await
            .is_err()
    );

    Ok(())
}

#[tokio::test]
async fn test_one_field_does_not_exists() -> Result<()> {
    let connect_options = SqliteConnectOptions::new()
        .filename("file:test_db?mode=memory&cache=shared")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;
    for i in 1..30 {
        let _ = sqlx::query(
            r#"
    INSERT INTO users (username, email,password)
    VALUES (?, ?, ?)
    "#,
        )
        .bind(i.to_string())
        .bind(format!("faycel{}@bhk.com", i))
        .bind(i.to_string())
        .execute(&pool)
        .await?;
    }
    // Wait for insertions to be completed
    let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count_result, 29);

    let table = "users";
    #[derive(Debug, sqlx::FromRow, HasColumns, PartialEq)]
    struct User {
        id: i64,
        test: String,
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: 2,
        sort_key: "id".to_string(),
        sort: SortOrder::Asc,
        searched_value: Some("2@bhk".to_string()),
    };
    assert!(
        get_paginated_results_by_string::<User>(&pool, table, &pagination_params)
            .await
            .is_err()
    );

    Ok(())
}
