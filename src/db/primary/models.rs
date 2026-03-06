use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipaDoc")]
use utoipa::{IntoParams, ToSchema};

/// Represents the sorting order (ascending or descending).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Sort results in ascending order (e.g., A-Z, 0-9).
    Asc = 1,
    /// Sort results in descending order (e.g., Z-A, 9-0).
    Desc = -1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(IntoParams, ToSchema))]
#[cfg_attr(feature = "utoipaDoc", into_params(parameter_in = Query))]
pub(crate) struct PaginationParams {
    /// The number of items to skip before returning results.
    pub skip: i64,
    /// The number of items per page (-1 for no limit).
    pub nbr: i64,
    /// The column used for sorting the results.
    pub sort_key: String,
    /// The sorting order (`Asc` or `Desc`).
    pub sort: SortOrder,
    /// An optional search term to filter results.
    pub searched_value: Option<String>,
}

// pub struct PaginationParams<Key>
// where
//     Key: AsRef<str> + Display + Serialize,
// {
//     /// The number of items to skip before returning results.
//     pub skip: i64,
//     /// The number of items per page (-1 for no limit).
//     pub nbr: i64,
//     /// The column used for sorting the results.
//     #[cfg_attr(feature = "utoipaDoc", schema(value_type = String))]
//     pub sort_key: Key,
//     /// The sorting order (`Asc` or `Desc`).
//     pub sort: SortOrder,
//     /// An optional search term to filter results.
//     pub searched_value: Option<String>,
// }

/// Represents the paginated response.
///
/// This struct contains the total number of items available and
/// the current page of results.
///
/// # Fields
///
/// * `total` - The total number of items matching the query.
/// * `items` - The current page of items.
///
/// # Example
///
/// ```
/// use db_handler::db::primary::models::PaginatedResponse;
///
/// let response: PaginatedResponse<String> = PaginatedResponse {
///     total: 100,
///     items: vec!["Item 1".to_string(), "Item 2".to_string()],
/// };
/// ```
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
pub struct PaginatedResponse<T> {
    /// The total number of items matching the query.
    pub total: i64,
    /// The current page of items.
    pub items: Vec<T>,
}
