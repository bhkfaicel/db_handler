use serde::{Deserialize, Serialize};
use std::fmt::Display;
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

/// Generic pagination parameters shared by every domain repository.
///
/// `K` is the sort-key type. Domains instantiate it with their typed
/// sort-key enum (`SensorSortKey`, `UserSortKey`), whose `AsRef<str>` /
/// `Display` implementations render the SQL column name, so no per-domain
/// copy of this model and no field-by-field conversion at service
/// boundaries is needed. `String` satisfies the bounds directly for
/// callers that have no typed key (unit tests, ad-hoc queries).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(IntoParams, ToSchema))]
#[cfg_attr(feature = "utoipaDoc", into_params(parameter_in = Query))]
// The sort key is rendered as its string form (`AsRef<str>`/`Display`),
// so no utoipa trait bound is imposed on `K`: removing the ToSchema
// derive auto-bounds lets the schema be generated for any `K` that
// satisfies the struct's own bounds.
#[cfg_attr(feature = "utoipaDoc", schema(bound = ""))]
pub struct PaginationParams<K>
where
    K: AsRef<str> + Display + Serialize,
{
    /// The number of items to skip before returning results.
    pub skip: i64,
    /// The number of items per page (-1 for no limit).
    pub nbr: i64,
    /// The column used for sorting the results.
    #[cfg_attr(feature = "utoipaDoc", schema(value_type = String))]
    #[cfg_attr(feature = "utoipaDoc", param(value_type = String))]
    pub sort_key: K,
    /// The sorting order (`Asc` or `Desc`).
    pub sort: SortOrder,
    /// An optional search term to filter results.
    pub searched_value: Option<String>,
}

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
