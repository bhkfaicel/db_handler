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
/// Line-protocol rendering happens exclusively through InfluxDB's
/// `DataPoint::builder`, which performs the required escaping of the
/// tag and field values; no manual formatter is kept here.
///
/// # Example
///
/// Build the InfluxDB point exactly like the write path does. The
/// example is `ignore` rather than a compiled doctest: `SensorDataPoint`
/// lives in a crate-private module, so no import path for it can link
/// from an external doctest crate.
///
/// ```ignore
/// use db_handler::history::sensors::model::{SensorDataPoint, SensorFields, SensorTags};
/// use db_handler::sensors::model::ProductType;
/// use chrono::Utc;
/// use influxdb2::models::{DataPoint, FieldValue};
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
/// let point = SensorDataPoint {
///     measurement: "sensor_data".to_string(),
///     tags,
///     fields,
///     timestamp: Utc::now(),
/// };
///
/// let datapoint = DataPoint::builder(point.measurement.as_str())
///     .tag("sensor_mac", point.tags.sensor_mac.as_str())
///     .field(
///         "is_open",
///         FieldValue::from(matches!(
///             point.fields,
///             SensorFields::Contact { is_open: true, .. }
///         )),
///     )
///     .build()
///     .expect("a valid data point");
/// let _ = datapoint;
/// ```
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
