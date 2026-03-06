use crate::common::helpers::config::HomeIotConfig;
use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::{
    env,
    fs::{self},
    path::PathBuf,
    time::Duration,
};
/// Initializes and returns a connection pool to a SQLite database.
///
/// Reads the database URL from a configuration file specified by the `CONFIG_FILE`
/// environment variable. Supports both in-memory and file-based databases, enables
/// foreign-key enforcement, configures shared cache or memory journal as appropriate,
/// and applies any pending migrations automatically (errors are ignored in `test` mode).
///
/// # Environment Variables
///
/// * `CONFIG_FILE` – Path to a JSON file containing a `db_url` field.
///
/// # Behavior
///
/// - If `db_url == ":memory:"`, opens an in-memory database with shared cache
///   (`file::memory:?cache=shared`) and uses `MEMORY` journal mode.
/// - Otherwise, opens (or creates) the file at `db_url` with foreign keys enabled
///   and shared cache.
///
/// # Returns
///
/// * `Ok(SqlitePool)` – Connection pool created and migrations run successfully.
/// * `Err(anyhow::Error)` – If a connection or migration error occurs (e.g., `CONFIG_FILE` not set, cannot be opened, or contains invalid JSON).
///
/// # Example
///
/// no_run
/// use db_handler::db::connection::Connection;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = Connection::new().await?;
/// let pool = connection.primary_db_client();
/// # Ok(())
/// # }
///

pub(crate) async fn get_primary_db_client() -> Result<SqlitePool> {
    let migrations: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");
    let config = HomeIotConfig::new().context("Failed to load configuration")?;
    let database_url = config.primary_db_url();
    let connect_options = if database_url == ":memory:" {
        SqliteConnectOptions::new()
            .filename("file::memory:?cache=shared") // Correct usage of SQLite shared in-memory database
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory)
    } else {
        // Normalize possible DSN forms like "sqlite:/abs/path" or "sqlite:///abs/path"
        let db_path = if let Some(stripped) = database_url.strip_prefix("sqlite:") {
            // Remove optional leading slashes after the scheme to get an absolute path
            // e.g. sqlite:///Users/... -> /Users/...
            //      sqlite:/Users/...   -> /Users/...
            let path_candidate = stripped.trim_start_matches('/');
            // Prepend '/' to ensure absolute path if original had it
            if stripped.starts_with('/') {
                format!("/{}", path_candidate)
            } else {
                path_candidate.to_string()
            }
        } else {
            database_url.clone()
        };

        // Ensure absolute path if a relative path is provided
        let db_path: PathBuf = {
            let p = PathBuf::from(&db_path);
            if p.is_absolute() {
                p
            } else {
                env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(p)
            }
        };

        // Ensure parent directory exists to avoid SQLITE_CANTOPEN (code 14)
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }

        SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .foreign_keys(true)
            .shared_cache(true)
            .pragma("key", "test")
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;

    // Sort migrations by filename to ensure correct execution order
    let mut migration_files: Vec<_> = migrations.files().collect();
    migration_files.sort_by_key(|f| f.path().to_string_lossy().to_string());
    for file in migration_files {
        if let Some(content) = file.contents_utf8() {
            sqlx::query(content).execute(&pool).await?;
            println!("file migration {} Ok", file.path().display());
        }
    }
    Ok(pool)
}
