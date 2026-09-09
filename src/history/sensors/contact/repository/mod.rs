use chrono::{DateTime, Utc};
use std::fmt::Write;
use std::sync::Arc;

use crate::db::history::connection::HistoryDbClient;
use crate::history::sensors::{
    error::{Result, SensorHistoryError},
    model::{fields::VibrationCount, point::SensorDataPoint, SensorFields, SensorTags},
};
use crate::sensors::model::ProductType;
pub(crate) struct Repository {
    client: Arc<HistoryDbClient>,
    bucket: String,
}

impl Repository {
    /// Creates a new instance of the repository with an InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - History database client
    /// * `bucket` - Name of the InfluxDB bucket history writes target,
    ///   taken from the configuration file
    ///
    /// # Returns
    ///
    /// A new instance of the repository initialized with the provided
    /// client and bucket name.
    pub(crate) fn new(client: Arc<HistoryDbClient>, bucket: String) -> Self {
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
        // The MAC is validated here (constructor): a malformed address
        // fails the write before any network traffic happens.
        let tags = SensorTags::new(ProductType::ContactSensor, sensor_id, sensor_mac.clone())?;
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
        let line_protocol = render_line_protocol(&point)?;
        log::debug!("Line protocol created successfully, writing to InfluxDB...");
        match self.client.write(&bucket, &line_protocol).await {
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

/// Converts a sensor point to InfluxDB line protocol.
fn render_line_protocol(point: &SensorDataPoint) -> Result<String> {
    let mut line = format!(
        "{},product_type={},sensor_id={},sensor_mac={} ",
        escape_key(&point.measurement),
        escape_key(&point.tags.product_type.to_string()),
        point.tags.sensor_id,
        escape_key(&point.tags.sensor_mac)
    );
    let mut fields = Vec::new();

    // Add fields based on the sensor type.
    match &point.fields {
        SensorFields::Contact {
            is_open,
            vibration_count,
            sensor_timestamp,
        } => {
            fields.push(format!("is_open={is_open}"));
            if let Some(vc) = vibration_count {
                fields.push(format!("vibration_count={}i", vc.count));
                fields.push(format!("vibration_time_window={}i", vc.time_window));
                fields.push(format!(
                    "vibration_time_unit=\"{}\"",
                    escape_string(&vc.unit.to_string())
                ));
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
                fields.push(format!("sensor_timestamp={timestamp_ns}i"));
            }
        }
    }
    line.push_str(&fields.join(","));

    // Add timestamp (convert DateTime<Utc> to nanoseconds since epoch); an
    // out-of-range value is a data error reported as `InvalidFormat`, never
    // a silent epoch-0 write.
    let timestamp_ns = point.timestamp.timestamp_nanos_opt().ok_or_else(|| {
        SensorHistoryError::invalid_format(format!(
            "Point timestamp {} is outside the nanosecond range",
            point.timestamp
        ))
    })?;
    write!(line, " {timestamp_ns}").expect("writing to a String cannot fail");
    Ok(line)
}

/// Escapes measurement and tag keys according to InfluxDB line protocol.
fn escape_key(value: &str) -> String {
    escape_chars(value, |character| match character {
        '\\' => Some("\\\\"),
        ',' => Some("\\,"),
        ' ' => Some("\\ "),
        '=' => Some("\\="),
        _ => None,
    })
}

/// Escapes a string field according to InfluxDB line protocol.
fn escape_string(value: &str) -> String {
    escape_chars(value, |character| match character {
        '\\' => Some("\\\\"),
        '"' => Some("\\\""),
        _ => None,
    })
}

/// Escapes selected characters without allocating intermediate strings.
fn escape_chars(value: &str, replacement: impl Fn(char) -> Option<&'static str>) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if let Some(replacement) = replacement(character) {
            escaped.push_str(replacement);
        } else {
            escaped.push(character);
        }
    }
    escaped
}

#[cfg(test)]
mod tests;
