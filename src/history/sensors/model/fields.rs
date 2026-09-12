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
/// Field rendering happens in the history repository, which handles the
/// line-protocol quoting and escaping of field values.
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
