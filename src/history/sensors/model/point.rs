use chrono::{DateTime, Utc};

use crate::history::sensors::model::{fields::SensorFields, tags::SensorTags};

/// Represents a complete sensor data point for time-series databases.
///
/// A data point consists of:
/// - Measurement name (e.g., "sensor_data")
/// - Tags (indexed metadata)
/// - Fields (actual measured values)
/// - Timestamp
///
/// Line-protocol rendering is kept in the history repository so this model
/// remains independent of the selected HTTP transport.
///
/// # Example
///
/// `SensorDataPoint` is crate-private and is rendered by the history
/// repository immediately before the write request.
///
/// ```ignore
#[derive(Debug, Clone)]
pub struct SensorDataPoint {
    /// Measurement name (e.g., "sensor_data")
    pub measurement: String,
    /// Tags for filtering and grouping
    pub tags: SensorTags,
    /// Fields containing actual measured values
    pub fields: SensorFields,
    /// Timestamp of the measurement
    pub timestamp: DateTime<Utc>,
}

impl SensorDataPoint {
    /// Creates a new sensor data point with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `measurement` - Measurement name
    /// * `tags` - Sensor tags
    /// * `fields` - Sensor fields
    ///
    /// # Returns
    ///
    /// A new `SensorDataPoint` with the current timestamp.
    ///
    /// # Example
    ///
    /// The example is `ignore` rather than a compiled doctest:
    /// `SensorDataPoint` lives in a crate-private module, so no import
    /// path for it can link from an external doctest crate.
    ///
    /// ```ignore
    /// use db_handler::history::sensors::model::{SensorDataPoint, SensorFields, SensorTags};
    /// use db_handler::sensors::model::ProductType;
    ///
    /// let tags = SensorTags::new(
    ///     ProductType::ContactSensor,
    ///     123,
    ///     "AA:BB:CC:DD:EE:01".to_string(),
    /// )
    /// .expect("the MAC is well formed");
    ///
    /// let fields = SensorFields::Contact {
    ///     is_open: true,
    ///     vibration_count: None,
    ///     sensor_timestamp: None,
    /// };
    ///
    /// let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);
    /// let _ = point;
    /// ```
    pub fn new(measurement: String, tags: SensorTags, fields: SensorFields) -> Self {
        Self {
            measurement,
            tags,
            fields,
            timestamp: Utc::now(),
        }
    }
}
