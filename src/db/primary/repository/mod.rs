use crate::{
    db::primary::models::{PaginatedResponse, PaginationParams, SortOrder},
    drives::columns::HasColumns,
};
use sqlx::{FromRow, Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

async fn get_table_columns(
    pool: &Pool<Sqlite>,
    table_name: &str,
) -> Result<HashMap<String, String>, sqlx::Error> {
    let query = format!("PRAGMA table_info({})", table_name);
    let rows = sqlx::query(&query).fetch_all(pool).await?;

    let mut columns = HashMap::new();

    for row in rows {
        let name: String = row.get("name");
        let col_type: String = row.get("type");
        columns.insert(name, col_type);
    }

    Ok(columns)
}

pub(crate) async fn get_paginated_results_by_string<T>(
    pool: &Pool<Sqlite>,
    table: &str,
    pagination: &PaginationParams,
) -> Result<PaginatedResponse<T>, anyhow::Error>
where
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + HasColumns + Unpin + 'static,
{
    let map = T::column_map();
    let column_to_return: Vec<String> = map
        .keys()
        .map(|k| k.trim_start_matches("r#").to_string())
        .collect();
    let string_columns: Vec<String> = map
        .iter()
        .filter(|(_, col_type)| col_type.to_lowercase() == "string")
        .map(|(col_name, _)| col_name.clone())
        .collect();
    if string_columns.is_empty() {
        return Err(anyhow::anyhow!("No string columns found"));
    }
    // Verify that the `sort_key` is valid
    let db_map = get_table_columns(&pool, table).await?;
    let columns: Vec<String> = db_map.keys().cloned().collect();
    // if !columns.contains(&(pagination.sort_key.as_ref()).to_string()) {
    //     return Err(anyhow::anyhow!(
    //         "Invalid sort key `{}` for table `{}`",
    //         pagination.sort_key,
    //         table
    //     ));
    // }
    if !columns.contains(&(pagination.sort_key).to_string()) {
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
                count_query_sql = count_query_sql.bind(format!("%{}%", value)); // 🔥 bind pour chaque colonne
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
        pagination.sort_key,
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
mod tests;
