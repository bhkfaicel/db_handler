#[cfg(feature = "utoipaDoc")]
use utoipa::{IntoParams, ToSchema};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{db::primary::models::SortOrder, drives::columns::HasColumns};
use db_derive::HasColumns;

#[derive(Debug, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
pub(crate) struct InternalUser {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
}

impl InternalUser {
    pub(crate) fn new(
        id: u32,
        username: String,
        email: String,
        password_hash: Option<String>,
    ) -> Self {
        InternalUser {
            id,
            username,
            email,
            password_hash,
        }
    }
}

/// Represents a user with an id, username, email and password_hash.
#[derive(Debug, FromRow, HasColumns, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
}

impl User {
    /// Creates a new `User` instance.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the user.
    /// * `username` - username  of the user.
    /// * `email` - email  of the user.
    /// * `password_hash` - password_hash of the user.
    ///
    /// # Example
    ///
    /// ```
    /// use db_handler::users::model::User;
    ///
    /// let user = User::new(1, "faycel".to_string(), "faycelbhk@gmail.com".to_string());
    /// assert_eq!(user.id, 1);
    /// assert_eq!(user.username, "faycel");
    /// assert_eq!(user.email, "faycelbhk@gmail.com");
    /// ```
    pub fn new(id: u32, username: String, email: String) -> Self {
        User {
            id,
            username,
            email,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "utoipaDoc", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum UserSortKey {
    Id,
    UserName,
    Email,
}

impl AsRef<str> for UserSortKey {
    fn as_ref(&self) -> &str {
        match self {
            UserSortKey::Id => "id",
            UserSortKey::UserName => "username",
            UserSortKey::Email => "email",
        }
    }
}

impl std::fmt::Display for UserSortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserSortKey::Id => write!(f, "id"),
            UserSortKey::UserName => write!(f, "username"),
            UserSortKey::Email => write!(f, "email"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipaDoc", derive(IntoParams, ToSchema))]
#[cfg_attr(feature = "utoipaDoc", into_params(parameter_in = Query))]
pub struct PaginationParams {
    /// The number of items to skip before returning results.
    pub skip: i64,
    /// The number of items per page (-1 for no limit).
    pub nbr: i64,
    /// The column used for sorting the results.
    pub sort_key: UserSortKey,
    /// The sorting order (`Asc` or `Desc`).
    pub sort: SortOrder,
    /// An optional search term to filter results.
    pub searched_value: Option<String>,
}
