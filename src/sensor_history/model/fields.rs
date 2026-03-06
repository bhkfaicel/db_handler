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
/// use db_handler::sensor_history::model::SensorFields;
///
/// // For a contact sensor
/// let fields = SensorFields::Contact {
///     is_open: true,
///     vibration_count: None,
/// };
///
/// // For a temperature sensor
/// let fields = SensorFields::Temperature {
///     temperature: 22.5,
///     humidity: Some(45.0),
/// };
///
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorFields {
    /// Fields for contact sensors (open/close detection)
    Contact {
        /// Whether the sensor detects an open state
        is_open: bool,
        /// Optional vibration count measurement
        vibration_count: Option<VibrationCount>,
    },
    /// Fields for temperature sensors
    Temperature {
        /// Temperature value in Celsius
        temperature: f64,
        /// Optional humidity value in percentage
        humidity: Option<f64>,
    },
    /// Fields for vibration sensors
    Vibration {
        /// Vibration level (0.0 to 1.0)
        vibration_level: f64,
        /// Frequency in Hz
        frequency: f64,
    },
    /// Generic numeric value field
    Numeric {
        /// Generic numeric value
        value: f64,
    },
}

impl SensorFields {
    /// Converts fields to Line Protocol format (compatible with InfluxDB).
    ///
    /// # Returns
    ///
    /// A string in the format: `is_open=true` or `temperature=22.5,humidity=45.0`
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use db_handler::sensor_history::model::SensorFields;
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
    /// # let fields = SensorFields::Temperature {
    /// #     temperature: 22.5,
    /// #     humidity: Some(45.0),
    /// # };
    /// # let line_protocol = fields.to_line_protocol();
    /// # assert!(line_protocol.contains("temperature=22.5"));
    /// # assert!(line_protocol.contains("humidity=45.0"));
    ///
    pub fn to_line_protocol(&self) -> String {
        match self {
            SensorFields::Contact {
                is_open,
                vibration_count,
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
                parts.join(",")
            }
            SensorFields::Temperature {
                temperature,
                humidity,
            } => {
                // Format floats to always show at least one decimal place
                let mut parts = vec![format!("temperature={:.1}", temperature)];
                if let Some(h) = humidity {
                    parts.push(format!("humidity={:.1}", h));
                }
                parts.join(",")
            }
            SensorFields::Vibration {
                vibration_level,
                frequency,
            } => {
                format!(
                    "vibration_level={:.1},frequency={:.1}",
                    vibration_level, frequency
                )
            }
            SensorFields::Numeric { value } => {
                format!("value={:.1}", value)
            }
        }
    }
}
