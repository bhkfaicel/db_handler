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
        match self.client.write(&bucket, line_protocol).await {
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
    let mut line = String::with_capacity(160);
    write_escaped_key(&mut line, &point.measurement);
    write!(
        line,
        ",product_type={},sensor_id={},sensor_mac=",
        point.tags.product_type, point.tags.sensor_id
    )
    .expect("writing to a String cannot fail");
    write_escaped_key(&mut line, &point.tags.sensor_mac);
    line.push(' ');

    // Add fields based on the sensor type.
    match &point.fields {
        SensorFields::Contact {
            is_open,
            vibration_count,
            sensor_timestamp,
        } => {
            write!(line, "is_open={is_open}").expect("writing to a String cannot fail");
            if let Some(vc) = vibration_count {
                write!(
                    line,
                    ",vibration_count={}i,vibration_time_window={}i,vibration_time_unit=\"{}\"",
                    vc.count, vc.time_window, vc.unit
                )
                .expect("writing to a String cannot fail");
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
                write!(line, ",sensor_timestamp={timestamp_ns}i")
                    .expect("writing to a String cannot fail");
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
    write!(line, " {timestamp_ns}").expect("writing to a String cannot fail");
    Ok(line)
}

/// Escapes measurement and tag keys according to InfluxDB line protocol.
fn write_escaped_key(output: &mut String, value: &str) {
    write_escaped(output, value, |character| match character {
        '\\' => Some("\\\\"),
        ',' => Some("\\,"),
        ' ' => Some("\\ "),
        '=' => Some("\\="),
        _ => None,
    });
}

/// Returns an escaped key for direct formatter compatibility tests.
#[cfg(test)]
fn escape_key(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    write_escaped_key(&mut escaped, value);
    escaped
}

/// Escapes a string field according to InfluxDB line protocol.
#[cfg(test)]
fn escape_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    write_escaped_string(&mut escaped, value);
    escaped
}

/// Escapes a string field directly into the final line-protocol buffer.
#[cfg(test)]
fn write_escaped_string(output: &mut String, value: &str) {
    write_escaped(output, value, |character| match character {
        '\\' => Some("\\\\"),
        '"' => Some("\\\""),
        _ => None,
    });
}

/// Writes escaped characters without allocating intermediate strings.
fn write_escaped(
    output: &mut String,
    value: &str,
    replacement: impl Fn(char) -> Option<&'static str>,
) {
    for character in value.chars() {
        if let Some(replacement) = replacement(character) {
            output.push_str(replacement);
        } else {
            output.push(character);
        }
    }
}

#[cfg(test)]
mod tests;
