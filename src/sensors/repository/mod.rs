use crate::{
    db::primary::{
        models::{PaginatedResponse, PaginationParams},
        repository::{get_paginated_results_by_string, PrimaryDbRepository},
    },
    sensors::model::{ProductType, Sensor, Status},
};
use anyhow::{anyhow, Ok, Result};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::{str::FromStr, sync::Arc};

/// Repository for performing CRUD operations on `Sensor` entities.
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
        .await?;
        Ok(result.last_insert_rowid() as u32)
    }

    pub(crate) async fn get_by_id(&self, id: u32) -> Result<Option<Sensor>> {
        let result = sqlx::query!(
            "SELECT id, mac, status,sensor_type FROM sensors WHERE id = ?",
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| anyhow::anyhow!("Invalid status value"))?;
            let sensor_type: Option<ProductType> = record
                .sensor_type
                .and_then(|s| s.parse::<ProductType>().ok());
            Ok(Some(Sensor {
                id: record.id as u32,
                mac: record.mac,
                sensor_type: sensor_type,
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
        .await?;
        if let Some(record) = result {
            let status = Status::from_str(&record.status)
                .map_err(|_| anyhow::anyhow!("Invalid status value"))?;
            let sensor_type: Option<ProductType> = record
                .sensor_type
                .and_then(|s| s.parse::<ProductType>().ok());
            Ok(Some(Sensor {
                id: record
                    .id
                    .map(|id| id as u32)
                    .ok_or_else(|| anyhow::anyhow!("ID is null"))?,
                mac: record.mac,
                sensor_type: sensor_type,
                status,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn delete_by_id(&self, id: u32) -> Result<()> {
        sqlx::query!("DELETE FROM sensors WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_by_mac(&self, mac: &str) -> Result<()> {
        sqlx::query!("DELETE FROM sensors WHERE mac = ?", mac)
            .execute(&*self.pool)
            .await?;
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
            .await?;

        let mut sensors = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let mac: String = row.try_get("mac")?;
            let status_raw: String = row.try_get("status")?;
            let sensor_type: Option<String> = row.try_get("sensor_type")?;
            sensors.push(Sensor {
                id: u32::try_from(id).map_err(|e| anyhow!("sensor id out of range: {e}"))?,
                mac,
                sensor_type: sensor_type.and_then(|st| st.parse::<ProductType>().ok()),
                status: Status::from_str(&status_raw)
                    .map_err(|_| anyhow!("Invalid status value"))?,
            });
        }
        Ok(sensors)
    }

    pub(crate) async fn update_by_mac(
        &self,
        mac: &str,
        sensor_status: Option<Status>,
        sensor_type: Option<ProductType>,
    ) -> Result<u32> {
        let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE sensors SET ");
        let mut fields_updated = 0;
        if let Some(status) = sensor_status {
            if status == Status::InProgress {
                return Err(anyhow!("Cannot update sensor status to 'InProgress': this status is only set during sensor creation"));
            }
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
        if fields_updated == 0 {
            return Err(anyhow!(
                "At least one field (status or sensor_type) must be provided for update"
            ));
        }

        query_builder.push(" WHERE mac = ").push_bind(mac);
        let query = query_builder.build();
        let result = query
            .execute(&*self.pool)
            .await
            .map_err(|e| anyhow!("Database update failed: {}", e))?;

        Ok(result.rows_affected() as u32)
    }

    pub(crate) async fn find_by_string(
        &self,
        pagination_params: &PaginationParams,
    ) -> Result<PaginatedResponse<Sensor>, anyhow::Error> {
        get_paginated_results_by_string::<Sensor>(&*self.pool, "sensors", pagination_params).await
    }
}

#[cfg(test)]
mod tests;
