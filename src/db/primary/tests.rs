use crate::{
    db::primary::models::{PaginationParams, SortOrder},
    db::primary::repository::get_paginated_results_by_string,
    drives::columns::HasColumns,
};
use anyhow::Result;
use db_derive::HasColumns;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row,
};
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

/// Simulates two gateway boots against one file-backed database and asserts
/// the migration ledger makes the second boot a no-op: applied files are
/// skipped and rows created between boots survive untouched.
#[tokio::test]
async fn migrations_apply_exactly_once_across_boots() -> Result<()> {
    use crate::db::connection::Connection;
    use std::env;

    // Isolated file-backed database for this test only.
    let db_path =
        env::temp_dir().join(format!("dbh_migrations_once_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let config_path =
        env::temp_dir().join(format!("dbh_migrations_cfg_{}.json", std::process::id()));
    std::fs::write(
        &config_path,
        format!(
            r#"{{"primary_db_url":"sqlite://{}","history_db_url":"http://localhost:8086","history_db_org":"t","history_db_token":"t"}}"#,
            db_path.display()
        ),
    )?;

    // Environment variables are process-global: save and restore so this
    // test cannot poison later ones (single-threaded suite).
    let original = env::var("CONFIG_FILE").ok();
    env::set_var("CONFIG_FILE", &config_path);

    let cleanup = |original: &Option<String>| {
        match original {
            Some(v) => env::set_var("CONFIG_FILE", v),
            None => {
                let _ = env::remove_var("CONFIG_FILE");
            }
        }
        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_file(&db_path);
    };

    // Boot 1: fresh database, every migration applies.
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let ledger: i64 = sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations")
        .fetch_one(&*pool)
        .await?
        .get("n");
    assert!(
        ledger >= 4,
        "all embedded migrations recorded, got {ledger}"
    );

    // Live row created after deployment, like a sensor reporting state.
    sqlx::query("INSERT INTO sensors (mac, status) VALUES ('00:1B:44:11:3A:E1', 'Installed')")
        .execute(&*pool)
        .await?;
    let id: i64 = sqlx::query("SELECT id FROM sensors WHERE mac = '00:1B:44:11:3A:E1'")
        .fetch_one(&*pool)
        .await?
        .get("id");
    sqlx::query(
        "INSERT INTO open_close_sensors (sensor_id, sensor_mac, is_open, updated_at) \
         VALUES (?, '00:1B:44:11:3A:E1', 0, datetime('now'))",
    )
    .bind(id)
    .execute(&*pool)
    .await?;
    drop(pool);
    drop(connection);

    // Boot 2: same database file; the ledger must skip every migration.
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let open_close_rows: i64 = sqlx::query(
        "SELECT COUNT(*) AS n FROM open_close_sensors WHERE sensor_mac = '00:1B:44:11:3A:E1'",
    )
    .fetch_one(&*pool)
    .await?
    .get("n");
    assert_eq!(open_close_rows, 1, "second boot must NOT replay migrations");
    let ledger_after: i64 = sqlx::query("SELECT COUNT(*) AS n FROM schema_migrations")
        .fetch_one(&*pool)
        .await?
        .get("n");
    assert_eq!(ledger_after, ledger, "no migration may be recorded twice");

    drop(pool);
    drop(connection);
    cleanup(&original);
    Ok(())
}
