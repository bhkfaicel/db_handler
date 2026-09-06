use crate::{
    db::primary::{
        error::{CommonError, Result},
        models::{PaginatedResponse, PaginationParams},
        repository::{get_paginated_results_by_string, PrimaryDbRepository},
        service::map_sqlx_to_common_error,
    },
    sensors::model::{ProductType, Sensor, SensorSortKey, Status},
};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::{str::FromStr, sync::Arc};

/// Repository for performing CRUD operations on `Sensor` entities.
///
/// All methods return `CommonError`: raw sqlx errors are converted by the
/// single central conversion point, so a duplicate MAC surfaces as the
/// structured `CommonError::UniqueViolation { constraint: "sensors.mac" }`
/// instead of requiring callers to inspect error messages.
pub(crate) struct SensorRepository {
    pool: Arc<SqlitePool>,
}
impl PrimaryDbRepository for SensorRepository {
    fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
impl SensorRepository {
    pub(crate) async fn insert(&self, sensor: &Sensor) -> Result<u32> {
        let result = sqlx::query!(
            "INSERT INTO sensors (mac, status) VALUES (?, ?)",
            sensor.mac,
            sensor.status,
        )
        .execute(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        Ok(result.last_insert_rowid() as u32)
    }

    pub(crate) async fn get_by_id(&self, id: u32) -> Result<Option<Sensor>> {
        let result = sqlx::query!(
            "SELECT id, mac, status,sensor_type FROM sensors WHERE id = ?",
            id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;

        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| CommonError::Operation("Invalid status value".to_string()))?;
            let sensor_type: Option<ProductType> = record
                .sensor_type
                .and_then(|s| s.parse::<ProductType>().ok());
            Ok(Some(Sensor {
                id: record.id as u32,
                mac: record.mac,
                sensor_type,
                status,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_by_mac(&self, mac: &str) -> Result<Option<Sensor>> {
        let result = sqlx::query!(
            "SELECT id, mac, status,sensor_type FROM sensors WHERE mac = ?",
            mac
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_sqlx_to_common_error)?;
        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| CommonError::Operation("Invalid status value".to_string()))?;
            let sensor_type: Option<ProductType> = record
                .sensor_type
                .and_then(|s| s.parse::<ProductType>().ok());
            let id = record
                .id
                .ok_or_else(|| CommonError::Operation("ID is null".to_string()))?
                as u32;
            Ok(Some(Sensor {
                id,
                mac: record.mac,
                sensor_type,
                status,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn delete_by_id(&self, id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM sensors WHERE id = ?", id)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    pub(crate) async fn delete_by_mac(&self, mac: &str) -> Result<()> {
        sqlx::query!("DELETE FROM sensors WHERE mac = ?", mac)
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;
        Ok(())
    }

    /// Lists every sensor currently in the given status.
    ///
    /// Gateway boot logic uses it to find commissioning runs left unfinished
    /// (`InProgress`) so they can be granted one bounded pairing retry
    /// opportunity. Uses the runtime query API on purpose: adding a new
    /// compile-time macro would require regenerating the offline statement
    /// cache, while existing macros stay untouched.
    pub(crate) async fn list_by_status(&self, status: &Status) -> Result<Vec<Sensor>> {
        let rows = sqlx::query("SELECT id, mac, status, sensor_type FROM sensors WHERE status = ?")
            .bind(status.to_string())
            .fetch_all(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;

        let mut sensors = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id").map_err(map_sqlx_to_common_error)?;
            let mac: String = row.try_get("mac").map_err(map_sqlx_to_common_error)?;
            let status_raw: String = row.try_get("status").map_err(map_sqlx_to_common_error)?;
            let sensor_type: Option<String> = row
                .try_get("sensor_type")
                .map_err(map_sqlx_to_common_error)?;
            sensors.push(Sensor {
                id: u32::try_from(id)
                    .map_err(|e| CommonError::Operation(format!("sensor id out of range: {e}")))?,
                mac,
                sensor_type: sensor_type.and_then(|st| st.parse::<ProductType>().ok()),
                status: Status::from_str(&status_raw)
                    .map_err(|_| CommonError::Operation("Invalid status value".to_string()))?,
            });
        }
        Ok(sensors)
    }

    /// Applies the requested field updates to the sensor with the given MAC.
    ///
    /// Pure data access: the business rules (no update to `InProgress`, at
    /// least one field required) are enforced by `SensorService` before this
    /// method is ever called, so no validation lives here.
    pub(crate) async fn update_by_mac(
        &self,
        mac: &str,
        sensor_status: Option<Status>,
        sensor_type: Option<ProductType>,
    ) -> Result<u32> {
        let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE sensors SET ");
        let mut fields_updated = 0;
        if let Some(status) = sensor_status {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("status = ").push_bind(status);
            fields_updated += 1;
        }
        if let Some(sensor_type) = sensor_type {
            if fields_updated > 0 {
                query_builder.push(", ");
            }
            query_builder.push("sensor_type = ").push_bind(sensor_type);
            fields_updated += 1;
        }

        query_builder.push(" WHERE mac = ").push_bind(mac);
        // The service contract guarantees at least one field was provided
        // (empty updates are rejected there before any SQL is sent); the
        // built SET clause must therefore never be empty.
        debug_assert!(
            fields_updated > 0,
            "update_by_mac requires at least one field; enforce this in SensorService"
        );
        let query = query_builder.build();
        let result = query
            .execute(&*self.pool)
            .await
            .map_err(map_sqlx_to_common_error)?;

        Ok(result.rows_affected() as u32)
    }

    pub(crate) async fn find_by_string(
        &self,
        pagination_params: &PaginationParams<SensorSortKey>,
    ) -> Result<PaginatedResponse<Sensor>> {
        // The shared pagination helper still reports errors as `anyhow`
        // (its rework is out of this module's scope); carry the message as
        // an operation error so the repository keeps a single error type.
        // `T` is pinned by the `PaginatedResponse<Sensor>` return type;
        // the sort-key parameter `K` is inferred from `pagination_params`.
        get_paginated_results_by_string(&self.pool, "sensors", pagination_params)
            .await
            .map_err(|e| CommonError::Operation(e.to_string()))
    }
}

#[cfg(test)]
mod tests;
