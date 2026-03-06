use chrono::{DateTime, Utc};

use crate::sensor_history::model::{fields::SensorFields, tags::SensorTags};

/// Represents a complete sensor data point for time-series databases.
///
/// A data point consists of:
/// - Measurement name (e.g., "sensor_data")
/// - Tags (indexed metadata)
/// - Fields (actual measured values)
/// - Timestamp
///
/// # Example
///
/// no_run
/// use db_handler::sensor_history::model::{SensorDataPoint, SensorFields, SensorTags};
/// use db_handler::sensors::model::ProductType;
/// use chrono::Utc;
///
/// let tags = SensorTags::new(
///     ProductType::ContactSensor,
///     123,
///     "AA:BB:CC:DD:EE:01".to_string(),
/// );
///
/// let fields = SensorFields::Contact {
///     is_open: true,
///     vibration_count: None,
/// };
///
/// let point = SensorDataPoint {
///     measurement: "sensor_data".to_string(),
///     tags,
///     fields,
///     timestamp: Utc::now(),
/// };
///
/// # let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);
/// # let line_protocol = point.to_line_protocol();
///
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
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::sensor_history::model::{SensorDataPoint, SensorFields, SensorTags};
    /// # use crate::sensors::model::ProductType;
    /// #
    /// # let tags = SensorTags::new(
    /// #     ProductType::ContactSensor,
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// # );
    /// #
    /// # let fields = SensorFields::Contact {
    /// #     is_open: true,
    /// #     vibration_count: None,
    /// # };
    /// #
    /// # let point = SensorDataPoint::new(
    /// #     "sensor_data".to_string(),
    /// #     tags,
    /// #     fields,
    /// # );
    ///
    pub fn new(measurement: String, tags: SensorTags, fields: SensorFields) -> Self {
        Self {
            measurement,
            tags,
            fields,
            timestamp: Utc::now(),
        }
    }

    /// Creates a new sensor data point with a specific timestamp.
    ///
    /// # Arguments
    ///
    /// * `measurement` - Measurement name
    /// * `tags` - Sensor tags
    /// * `fields` - Sensor fields
    /// * `timestamp` - Timestamp of the measurement
    ///
    /// # Returns
    ///
    /// A new `SensorDataPoint` with the specified timestamp.
    pub fn with_timestamp(
        measurement: String,
        tags: SensorTags,
        fields: SensorFields,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            measurement,
            tags,
            fields,
            timestamp,
        }
    }

    /// Converts the data point to Line Protocol format (compatible with InfluxDB).
    ///
    /// # Returns
    ///
    /// A string in Line Protocol format:
    /// `measurement,tag1=value1,tag2=value2 field1=value1,field2=value2 timestamp`
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::sensor_history::model::{SensorDataPoint, SensorFields, SensorTags};
    /// # use crate::sensors::model::ProductType;
    /// # use chrono::Utc;
    /// #
    /// # let tags = SensorTags::new(
    /// #     ProductType::ContactSensor,
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// # );
    /// #
    /// # let fields = SensorFields::Contact {
    /// #     is_open: true,
    /// #     vibration_count: None,
    /// # };
    /// #
    /// # let point = SensorDataPoint {
    /// #     measurement: "sensor_data".to_string(),
    /// #     tags,
    /// #     fields,
    /// #     timestamp: Utc::now(),
    /// # };
    /// #
    /// # let line_protocol = point.to_line_protocol();
    /// # assert!(line_protocol.starts_with("sensor_data,"));
    /// # assert!(line_protocol.contains("is_open=true"));
    ///
    pub fn to_line_protocol(&self) -> String {
        let tags_str = self.tags.to_line_protocol();
        let fields_str = self.fields.to_line_protocol();
        let timestamp_ns = self.timestamp.timestamp_nanos_opt().unwrap_or(0);

        format!(
            "{},{} {} {}",
            self.measurement, tags_str, fields_str, timestamp_ns
        )
    }
}
