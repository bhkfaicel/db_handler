// Every item in this module serves the domain repositories, which only
// exist under the `sensors` or `users` features, so the imports are gated
// with the same condition: in a core-only build the module is empty and
// must not raise unused-import warnings.
#[cfg(any(feature = "sensors", feature = "users"))]
use crate::{
    db::primary::models::{PaginatedResponse, PaginationParams, SortOrder},
    drives::columns::HasColumns,
};
#[cfg(any(feature = "sensors", feature = "users"))]
use serde::Serialize;
#[cfg(any(feature = "sensors", feature = "users"))]
use sqlx::{FromRow, Pool, Sqlite, SqlitePool};
#[cfg(any(feature = "sensors", feature = "users"))]
use std::fmt::Display;
#[cfg(any(feature = "sensors", feature = "users"))]
use std::sync::Arc;

// The pagination infrastructure exists solely for the domain repositories,
// which are only compiled under the `sensors` or `users` features, so it is
// gated the same way: without a gate the core-only build reports it as dead
// code.
#[cfg(any(feature = "sensors", feature = "users"))]
pub(crate) async fn get_paginated_results_by_string<T, K>(
    pool: &Pool<Sqlite>,
    table: &str,
    pagination: &PaginationParams<K>,
) -> Result<PaginatedResponse<T>, anyhow::Error>
where
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + HasColumns + Unpin + 'static,
    K: AsRef<str> + Display + Serialize,
{
    // The model's column map, generated at compile time by `db_derive`, is
    // the single source of truth for the table columns: the schema is fixed
    // once the boot migrations have run, so the in-memory map is authoritative
    // and no per-call `PRAGMA table_info` round-trip is needed.
    let map = T::column_map();
    // Sorted so the generated SELECT column list is deterministic instead of
    // depending on `HashMap` iteration order.
    let mut column_to_return: Vec<String> = map.keys().cloned().collect();
    column_to_return.sort();
    let string_columns: Vec<String> = map
        .iter()
        .filter(|(_, col_type)| col_type.to_lowercase() == "string")
        .map(|(col_name, _)| col_name.clone())
        .collect();
    if string_columns.is_empty() {
        return Err(anyhow::anyhow!("No string columns found"));
    }
    // The sort key is validated against the same in-memory map (same fixed
    // schema assumption): a column absent from the model's column map can
    // never name a sortable column of the table.
    if !map.keys().any(|col| col == pagination.sort_key.as_ref()) {
        return Err(anyhow::anyhow!(
            "Invalid sort key `{}` for table `{}`",
            pagination.sort_key,
            table
        ));
    }

    let sort_order = match pagination.sort {
        SortOrder::Asc => "ASC",
        SortOrder::Desc => "DESC",
    };

    // Build the WHERE clause for the search
    let search_filter = match &pagination.searched_value {
        Some(_) => {
            let conditions: Vec<String> = string_columns
                .iter()
                .map(|col| format!("{} LIKE ? COLLATE NOCASE", col))
                .collect();
            format!("WHERE {}", conditions.join(" OR "))
        }
        None => "".to_string(),
    };
    // Get the total number of items

    let count_query = format!("SELECT count(*) FROM {} {}", table, search_filter);
    let count = match &pagination.searched_value {
        Some(value) => {
            let mut count_query_sql = sqlx::query_scalar::<_, i64>(&count_query);
            for _ in &string_columns {
                count_query_sql = count_query_sql.bind(format!("%{}%", value)); // one binding per string column
            }
            count_query_sql.fetch_one(pool).await?
        }
        None => {
            sqlx::query_scalar::<_, i64>(&count_query)
                .fetch_one(pool)
                .await?
        }
    };
    // Build the main query
    let query = format!(
        "SELECT {} FROM {} {} ORDER BY {} {} LIMIT ? OFFSET ?",
        column_to_return.join(", "),
        table,
        search_filter,
        pagination.sort_key.as_ref(),
        sort_order
    );
    let mut sql_query = sqlx::query_as::<_, T>(&query);
    if let Some(value) = &pagination.searched_value {
        for _ in string_columns {
            sql_query = sql_query.bind(format!("%{}%", value));
        }
    }
    sql_query = sql_query.bind(pagination.nbr).bind(pagination.skip);

    let items = sql_query.fetch_all(pool).await?;

    Ok(PaginatedResponse {
        total: count,
        items,
    })
}

// The pagination infrastructure exists solely for the domain repositories
// (only compiled under the `sensors` or `users` features), so the trait is
// gated the same way; every repository implementation lives inside those
// gated domains.
#[cfg(any(feature = "sensors", feature = "users"))]
pub(crate) trait PrimaryDbRepository: Sized {
    /// Creates a new instance of the repository with a SQLite connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - SQLite connection pool
    ///
    /// # Returns
    ///
    /// A new instance of the repository initialized with the provided connection.
    fn new(pool: Arc<SqlitePool>) -> Self;
}

#[cfg(test)]
// The repository tests exercise the pagination infrastructure, which only
// exists under the `sensors` or `users` features, so the module carries
// the same gate: both conditions must hold for it to compile.
#[cfg(any(feature = "sensors", feature = "users"))]
mod tests;
