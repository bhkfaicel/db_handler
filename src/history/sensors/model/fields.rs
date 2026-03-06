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
/// # Example
///
/// no_run
/// use db_handler::history::sensors::model::fields::SensorFields;
///
/// // For a contact sensor
/// let fields = SensorFields::Contact {
///     is_open: true,
///     vibration_count: None,
/// };
///
///
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

impl SensorFields {
    /// Converts fields to Line Protocol format (compatible with InfluxDB).
    ///
    /// # Returns
    ///
    /// A string in the format: `is_open=true` or `is_open=false`
    ///
    /// # Example
    ///
    /// no_run
    ///
    /// # use db_handler::history::sensors::model::fields::SensorFields;
    ///
    /// let fields = SensorFields::Contact {
    ///     is_open: true,
    ///     vibration_count: None,
    /// };
    /// # let fields = SensorFields::Contact {
    /// #     is_open: true,
    /// #     vibration_count: None,
    /// # };
    /// # let line_protocol = fields.to_line_protocol();
    /// # assert_eq!(line_protocol, "is_open=true");
    /// #
    pub fn to_line_protocol(&self) -> String {
        match self {
            SensorFields::Contact {
                is_open,
                vibration_count,
                sensor_timestamp,
            } => {
                let mut parts = vec![format!(
                    "is_open={}",
                    if *is_open { "true" } else { "false" }
                )];
                if let Some(vc) = vibration_count {
                    parts.push(format!("vibration_count={}", vc.count));
                    parts.push(format!("vibration_time_window={}", vc.time_window));
                    parts.push(format!("vibration_time_unit=\"{}\"", vc.unit));
                }
                if let Some(timestamp) = sensor_timestamp {
                    parts.push(format!(
                        "sensor_timestamp={}",
                        timestamp.timestamp_nanos_opt().unwrap_or(0)
                    ));
                }
                parts.join(",")
            }
        }
    }
}
