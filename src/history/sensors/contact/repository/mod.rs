use chrono::{DateTime, Utc};
use futures::stream;
use influxdb2::models::{DataPoint, FieldValue};

use crate::{
    db::history::{bucket_name::BucketName, repository::HistoryDbRepository},
    history::sensors::{
        error::{Result, SensorHistoryError},
        model::{fields::VibrationCount, point::SensorDataPoint, SensorFields, SensorTags},
    },
    sensors::model::ProductType,
};
use std::sync::Arc;

pub(crate) struct Repository {
    client: Arc<influxdb2::Client>,
}
impl HistoryDbRepository for Repository {
    fn new(client: Arc<influxdb2::Client>) -> Self {
        Self { client }
    }
}
impl Repository {
    pub async fn set_history(
        &self,
        sensor_id: u32,
        sensor_mac: String,
        is_open: bool,
        vibration_count: Option<VibrationCount>,
        sensor_timestamp: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let tags = SensorTags::new(ProductType::ContactSensor, sensor_id, sensor_mac.clone());
        let fields = SensorFields::Contact {
            is_open,
            vibration_count,
            sensor_timestamp,
        };
        let bucket = BucketName::Sensor.to_string();
        log::debug!(
            "Writing history data to InfluxDB - bucket: {}, sensor_id: {}, sensor_mac: {}, is_open: {}",
            bucket,
            sensor_id,
            sensor_mac,
            is_open
        );
        let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);
        let datapoint = convert_to_influx_datapoint(&point)?;
        log::debug!("DataPoint created successfully, writing to InfluxDB...");
        match self
            .client
            .write(&bucket, stream::iter(vec![datapoint]))
            .await
        {
            Ok(_) => {
                log::info!(
                    "Successfully wrote history data to InfluxDB - bucket: {}, sensor_id: {}, sensor_mac: {}",
                    bucket,
                    sensor_id,
                    sensor_mac
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to write data point to InfluxDB - bucket: {}, sensor_id: {}, sensor_mac: {}, error: {}",
                    bucket, sensor_id, sensor_mac, e
                );
                log::error!("{}", error_msg);
                Err(SensorHistoryError::write(error_msg))
            }
        }
    }
}

/// Converts a SensorDataPoint to an InfluxDB DataPoint.
fn convert_to_influx_datapoint(point: &SensorDataPoint) -> Result<DataPoint> {
    let mut builder = DataPoint::builder(point.measurement.clone());

    // Add tags
    builder = builder.tag("product_type", point.tags.product_type.to_string());
    builder = builder.tag("sensor_id", point.tags.sensor_id.to_string());
    builder = builder.tag("sensor_mac", point.tags.sensor_mac.clone());

    // Add fields based on type
    match &point.fields {
        SensorFields::Contact {
            is_open,
            vibration_count,
            sensor_timestamp,
        } => {
            builder = builder.field("is_open", FieldValue::from(*is_open));
            if let Some(vc) = vibration_count {
                builder = builder.field("vibration_count", FieldValue::from(vc.count as i64));
                builder = builder.field(
                    "vibration_time_window",
                    FieldValue::from(vc.time_window as i64),
                );
                builder =
                    builder.field("vibration_time_unit", FieldValue::from(vc.unit.to_string()));
            }
            if let Some(timestamp) = sensor_timestamp {
                builder = builder.field(
                    "sensor_timestamp",
                    FieldValue::from(timestamp.timestamp_nanos_opt().unwrap_or(0)),
                );
            }
        }
    }

    // Add timestamp (convert DateTime<Utc> to nanoseconds since epoch)
    let timestamp_ns = point.timestamp.timestamp_nanos_opt().unwrap_or(0);
    builder = builder.timestamp(timestamp_ns);

    builder.build().map_err(|e| {
        SensorHistoryError::invalid_format(format!("Failed to build DataPoint: {}", e))
    })
}
