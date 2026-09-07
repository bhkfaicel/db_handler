use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Time unit for vibration count measurements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeUnit {
    /// Seconds
    Seconds,
    /// Minutes
    Minutes,
    /// Hours
    Hours,
}

impl std::fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeUnit::Seconds => write!(f, "seconds"),
            TimeUnit::Minutes => write!(f, "minutes"),
            TimeUnit::Hours => write!(f, "hours"),
        }
    }
}

/// Vibration count measurement for contact sensors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VibrationCount {
    /// Number of vibrations detected
    pub count: u16,
    /// Time window for the count
    pub time_window: u16,
    /// Unit of the time window
    pub unit: TimeUnit,
}

/// Fields for time-series sensor data points.
///
/// Fields represent the actual measured values. Unlike tags, fields
/// are not indexed and can change frequently. Different sensor types
/// will have different field structures.
///
/// Field rendering happens exclusively through InfluxDB's
/// `DataPoint::builder` (the write path in this crate), which handles
/// the line-protocol quoting and escaping of field values.
///
/// # Example
///
/// Add the measured values exactly like the write path does:
///
/// no_run
///
/// ```
/// use db_handler::history::sensors::model::fields::SensorFields;
/// use influxdb2::models::{DataPoint, FieldValue};
///
/// // For a contact sensor
/// let fields = SensorFields::Contact {
///     is_open: true,
///     vibration_count: None,
///     sensor_timestamp: None,
/// };
///
/// let mut builder = DataPoint::builder("sensor_data");
/// if let SensorFields::Contact { is_open, .. } = fields {
///     builder = builder.field("is_open", FieldValue::from(is_open));
/// }
/// let point = builder.build().expect("a valid data point");
/// let _ = point;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorFields {
    /// Fields for contact sensors (open/close detection)
    Contact {
        /// Whether the sensor detects an open state
        is_open: bool,
        /// Optional vibration count measurement
        vibration_count: Option<VibrationCount>,
        /// Optional sensor timestamp
        sensor_timestamp: Option<DateTime<Utc>>,
    },
}
