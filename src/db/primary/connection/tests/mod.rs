//! Tests for the primary SQLite connection factory's pragma configuration.
//!
//! The file branch must run in WAL journal mode with `synchronous` still at
//! the SQLite default FULL, and the `:memory:` branch must keep yielding a
//! working pool. Both branches are exercised through the production factory
//! (`Connection::new`), so the assertions hold against the real options
//! construction instead of a copy of it.

use crate::db::connection::Connection;
use anyhow::Result;
use sqlx::Row;
use std::env;

/// Drives both branches of `get_primary_db_client` through `Connection::new`
/// and asserts the configuration that matters under concurrent writes:
///
/// - a temporary file database must report `PRAGMA journal_mode = wal`
///   (readers no longer block the committing writer) and
///   `PRAGMA synchronous = 2` (FULL — the deliberate durability decision;
///   NORMAL is an unapplied user decision and must stay that way);
/// - the `:memory:` branch must still return a usable pool. Its migration
///   and shared-cache semantics are already covered by the `db::primary`
///   suite, so only liveness is asserted here.
///
/// One single test on purpose (liveness coverage): every `CONFIG_FILE`
/// mutation is still serialized by the crate's shared test guard, which
/// also restores the previous value on drop.
#[tokio::test]
async fn file_db_pragmas_and_memory_branch_still_work() -> Result<()> {
    // Isolated temporary file database plus a matching config file, mirroring
    // the migration-ledger test's setup (temp dir keyed by process id).
    let db_path = env::temp_dir().join(format!("dbh_wal_pragma_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let config_path = env::temp_dir().join(format!("dbh_wal_cfg_{}.json", std::process::id()));
    std::fs::write(
        &config_path,
        format!(
            r#"{{"primary_db_url":"sqlite://{}","history_db_url":"http://localhost:8086","history_db_org":"t","history_db_token":"t"}}"#,
            db_path.display()
        ),
    )?;

    // Environment variables are process-global: the shared test guard
    // serializes the mutation across parallel tests and restores the
    // previous value on drop.
    let guard = crate::test_env::ConfigFileGuard::acquire();
    guard.set(Some(config_path.to_str().unwrap()));

    // File branch through the production factory: pooled connections carry
    // the factory's connect options, so any connection observes the pragmas.
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let row = sqlx::query("PRAGMA journal_mode").fetch_one(&*pool).await?;
    let journal_mode: String = row.get(0);
    assert_eq!(
        journal_mode.to_ascii_lowercase(),
        "wal",
        "the file branch must enable WAL so readers do not block the writer"
    );
    let row = sqlx::query("PRAGMA synchronous").fetch_one(&*pool).await?;
    let synchronous: i64 = row.get(0);
    assert_eq!(
        synchronous, 2,
        "synchronous must stay at FULL (2); NORMAL is deliberately not applied"
    );

    // The `:memory:` branch must still yield a working pool: one liveness
    // query through a freshly built pool is enough (its migration and
    // shared-cache behavior is proven by the existing db::primary suite).
    let memory_config = env::temp_dir().join(format!("dbh_wal_mem_{}.json", std::process::id()));
    std::fs::write(
        &memory_config,
        r#"{"primary_db_url":":memory:","history_db_url":"http://localhost:8086","history_db_org":"t","history_db_token":"t"}"#,
    )?;
    guard.set(Some(memory_config.to_str().unwrap()));
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client();
    let row = sqlx::query("SELECT 1 AS ok").fetch_one(&*pool).await?;
    assert_eq!(
        row.get::<i64, _>("ok"),
        1,
        "the :memory: branch must keep producing a usable pool"
    );

    // Drop the guard: it restores the previous CONFIG_FILE value.
    drop(guard);
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_file(&memory_config);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
    Ok(())
}
