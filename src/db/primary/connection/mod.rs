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
/// foreign-key enforcement, configures the journal mode as appropriate (WAL for
/// file databases, MEMORY plus a shared-cache URI for in-memory ones), and applies
/// any pending migrations automatically (errors are ignored in `test` mode).
///
/// # Environment Variables
///
/// * `CONFIG_FILE` – Path to a JSON file containing a `primary_db_url` field.
///
/// # Behavior
///
/// - If `primary_db_url == ":memory:"`, opens an in-memory database with shared cache
///   (`file::memory:?cache=shared`) and uses `MEMORY` journal mode.
/// - Otherwise, opens (or creates) the file at `primary_db_url` with foreign keys enabled
///   and WAL journal mode.
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
            // WAL journal mode: readers work on snapshots and no longer block
            // the writer, so concurrent sensor upserts, key-rotation
            // transactions and nonce inserts do not queue behind a committing
            // writer (rollback-journal commits need exclusive access while
            // active readers drain, which turns contention into latency
            // spikes under burst load). WAL removes that commit-phase
            // exclusivity.
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        // `synchronous` stays at the SQLite default FULL (pinned by the
        // connection tests): switching to NORMAL could roll back
        // security-relevant commits (anti-replay nonces, refresh tokens)
        // after a power loss, so it remains an explicit user decision
        // that has deliberately NOT been applied.
        //
        // `shared_cache` is deliberately NOT set on the file branch: a
        // pool of connections sharing one page cache contends on
        // table-level locks (SQLITE_LOCKED) instead of progressing
        // concurrently. WAL plus the 5 s busy_timeout cover residual
        // write contention. The single shared in-memory database keeps
        // its `cache=shared` URI in the `:memory:` branch above, the one
        // setup where shared cache is the right choice.
        // No at-rest encryption pragma here: no SQLCipher backend exists in
        // the dependency graph, so an encryption pragma would be a silent
        // no-op and a configured key a false guarantee. At-rest protection
        // of the SQLite file (file permissions, system-level encryption)
        // is the deployer's responsibility; see the configuration section
        // of the README.
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;

    // Schema migration ledger: records applied files so later boots skip
    // them instead of replaying (replaying is wasted work and unsafe for
    // non-idempotent DDL such as table rebuilds). Legacy databases that
    // pre-date the ledger replay their idempotent files exactly once more,
    // then get recorded like any other.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (\
             name TEXT PRIMARY KEY, \
             applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
    )
    .execute(&pool)
    .await?;

    // Sort migrations by filename to ensure correct execution order.
    // Only `.sql` files are executed; other files (e.g. README.md) are ignored.
    let mut migration_files: Vec<_> = migrations
        .files()
        .filter(|f| {
            f.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
        })
        .collect();
    migration_files.sort_by_key(|f| f.path().to_string_lossy().to_string());
    for file in migration_files {
        let Some(content) = file.contents_utf8() else {
            continue;
        };
        let name = file
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .with_context(|| format!("unnamed migration file {}", file.path().display()))?;
        let already_applied = sqlx::query("SELECT 1 FROM schema_migrations WHERE name = ?")
            .bind(&name)
            .fetch_optional(&pool)
            .await?
            .is_some();
        if already_applied {
            continue;
        }
        // Each migration runs inside its own transaction together with its
        // ledger row: an interrupted boot can never persist a half-applied
        // schema change.
        let mut tx = pool.begin().await?;
        sqlx::query(content).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO schema_migrations (name) VALUES (?)")
            .bind(&name)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        // Library code logs instead of printing: the application decides
        // where migration progress output goes.
        log::info!("migration {name} applied");
    }
    Ok(pool)
}

#[cfg(test)]
mod tests;
