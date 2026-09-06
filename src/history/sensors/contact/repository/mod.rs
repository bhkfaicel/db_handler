use chrono::{DateTime, Utc};
use futures::stream;
use influxdb2::models::{DataPoint, FieldValue};

use crate::history::sensors::{
    error::{Result, SensorHistoryError},
    model::{fields::VibrationCount, point::SensorDataPoint, SensorFields, SensorTags},
};
use crate::sensors::model::ProductType;
use std::sync::Arc;

pub(crate) struct Repository {
    client: Arc<influxdb2::Client>,
    bucket: String,
}

impl Repository {
    /// Creates a new instance of the repository with an InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - InfluxDB client
    /// * `bucket` - Name of the InfluxDB bucket history writes target,
    ///   taken from the configuration file
    ///
    /// # Returns
    ///
    /// A new instance of the repository initialized with the provided
    /// client and bucket name.
    pub(crate) fn new(client: Arc<influxdb2::Client>, bucket: String) -> Self {
        Self { client, bucket }
    }

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
        // The bucket comes from the configuration file through the
        // construction chain; a clone per write keeps `&self` borrow-free.
        let bucket = self.bucket.clone();
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
                // Hot path: this line runs on every successful sensor event,
                // so success details stay at debug level (info would pay a
                // format + I/O per event on the gateway); failures below
                // remain error-level.
                log::debug!(
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
                // An out-of-range timestamp must fail loudly: silently
                // writing epoch 0 would corrupt the history record.
                let timestamp_ns = timestamp.timestamp_nanos_opt().ok_or_else(|| {
                    SensorHistoryError::invalid_format(format!(
                        "Sensor timestamp {} is outside the nanosecond range",
                        timestamp
                    ))
                })?;
                builder = builder.field("sensor_timestamp", FieldValue::from(timestamp_ns));
            }
        }
    }

    // Add timestamp (convert DateTime<Utc> to nanoseconds since epoch); an
    // out-of-range value is a data error reported as `InvalidFormat`, never
    // a silent epoch-0 write.
    let timestamp_ns = point.timestamp.timestamp_nanos_opt().ok_or_else(|| {
        SensorHistoryError::invalid_format(format!(
            "Point timestamp {} is outside the nanosecond range",
            point.timestamp
        ))
    })?;
    builder = builder.timestamp(timestamp_ns);

    builder.build().map_err(|e| {
        SensorHistoryError::invalid_format(format!("Failed to build DataPoint: {}", e))
    })
}

#[cfg(test)]
mod tests;
