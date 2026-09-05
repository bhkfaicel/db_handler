use crate::common::helpers::controller::{hash_password, verify_password};
use crate::db::primary::{
    models::{PaginatedResponse, PaginationParams},
    repository::{get_paginated_results_by_string, PrimaryDbRepository},
};
use anyhow::{anyhow, Result};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::sync::Arc;

use crate::users::model::User;

use super::model::InternalUser;

/// Repository for performing CRUD operations on `User` entities.
pub(crate) struct UserRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for UserRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl UserRepository {
    pub(crate) async fn insert(&self, user: &InternalUser) -> Result<u32> {
        let result = sqlx::query!(
            "INSERT INTO users (email, username,password_hash) VALUES (?, ?,?)",
            user.email,
            user.username,
            user.password_hash
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }

    pub(crate) async fn update_by_id(
        &self,
        id: u32,
        username: Option<String>,
        email: Option<String>,
        password: Option<String>,
    ) -> Result<u32> {
        let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE users SET ");
        let mut fields_updated = 0;

        if let Some(username) = username {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("username = ").push_bind(username);
            fields_updated += 1;
        }

        if let Some(email) = email {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("email = ").push_bind(email);
            fields_updated += 1;
        }

        if let Some(password) = password {
            let hash =
                hash_password(&password).map_err(|e| anyhow!("Password hashing failed: {}", e))?;

            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("password_hash = ").push_bind(hash);
            fields_updated += 1;
        }

        if fields_updated == 0 {
            return Err(anyhow!(
                "At least one field (username, email, or password) must be provided for update"
            ));
        }

        query_builder.push(" WHERE id = ").push_bind(id);

        let query = query_builder.build();
        let result = query
            .execute(&*self.pool)
            .await
            .map_err(|e| anyhow!("Database update failed: {}", e))?;

        Ok(result.rows_affected() as u32)
    }

    pub(crate) async fn get_by_id(&self, id: u32) -> Result<Option<User>> {
        let result = sqlx::query!("SELECT id, username, email FROM users WHERE id = ?", id)
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(record) = result {
            Ok(Some(User {
                id: record.id as u32,
                username: record.username,
                email: record.email,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_by_email_and_password(
        &self,
        email: String,
        password: String,
    ) -> Result<Option<User>> {
        let result = sqlx::query!(
            "SELECT id, username, email,password_hash FROM users WHERE email = ?",
            email
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(record) = result {
            match verify_password(&password, &record.password_hash) {
                Ok(true) => {
                    let id = record
                        .id
                        .ok_or_else(|| anyhow::anyhow!("Missing id in database record"))?
                        as u32;
                    Ok(Some(User {
                        id,
                        username: record.username,
                        email: record.email,
                    }))
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn delete_by_id(&self, id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM users WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn find_by_string(
        &self,
        pagination_params: &PaginationParams,
    ) -> Result<PaginatedResponse<User>, anyhow::Error> {
        get_paginated_results_by_string::<User>(&self.pool, "users", pagination_params).await
    }
}

#[cfg(test)]
mod tests;
