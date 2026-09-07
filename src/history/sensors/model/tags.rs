use crate::history::sensors::error::{Result, SensorHistoryError};
use crate::sensors::model::ProductType;
use serde::{Deserialize, Serialize};

/// Tags for time-series sensor data points.
///
/// Tags are indexed metadata used for filtering and grouping data.
/// In time-series databases, tags should contain values that are frequently queried
/// but don't change frequently.
///
/// Line-protocol rendering happens exclusively through InfluxDB's
/// `DataPoint::builder`, which performs the required escaping of the tag
/// values (`\`, `,`, `=`, space); no manual formatter is kept here.
///
/// # Example
///
/// Build the point exactly like the write path does. The example is
/// `ignore` rather than a compiled doctest: `SensorTags` lives in a
/// crate-private module, so no import path for it can link from an
/// external doctest crate.
///
/// ```ignore
/// use db_handler::history::sensors::model::SensorTags;
/// use db_handler::sensors::model::ProductType;
/// use influxdb2::models::DataPoint;
///
/// let tags = SensorTags::new(
///     ProductType::ContactSensor,
///     123,
///     "AA:BB:CC:DD:EE:01".to_string(),
/// )
/// .expect("the MAC is well formed");
///
/// let point = DataPoint::builder("sensor_data")
///     .tag("product_type", ProductType::ContactSensor.to_string())
///     .tag("sensor_id", tags.sensor_id.to_string())
///     .tag("sensor_mac", tags.sensor_mac.as_str())
///     .build()
///     .expect("a valid data point");
/// let _ = point;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensorTags {
    /// Type of product/sensor (e.g., ContactSensor)
    pub product_type: ProductType,
    /// Unique sensor identifier
    pub sensor_id: u32,
    /// Sensor MAC address. The format (six hexadecimal pairs separated
    /// by `:`, case-insensitive) is enforced both by the constructor and
    /// here at deserialization: the fields are public, so a constructor
    /// check alone could be bypassed by building the struct directly or
    /// by deserializing an untrusted payload.
    #[serde(deserialize_with = "crate::common::helpers::mac::deserialize")]
    pub sensor_mac: String,
}

impl SensorTags {
    /// Creates a new `SensorTags` instance.
    ///
    /// The sensor MAC must be six hexadecimal pairs separated by `:`
    /// (case-insensitive, e.g. `AA:BB:CC:DD:EE:01`); any other value is
    /// rejected as `SensorHistoryError::InvalidFormat` before it can
    /// reach the time-series write path.
    ///
    /// # Arguments
    ///
    /// * `product_type` - Type of the sensor product
    /// * `sensor_id` - Unique sensor identifier
    /// * `sensor_mac` - Sensor MAC address
    ///
    /// # Returns
    ///
    /// A new `SensorTags` instance, or an `InvalidFormat` error when the
    /// MAC is malformed.
    ///
    /// # Example
    ///
    /// The example is `ignore` rather than a compiled doctest:
    /// `SensorTags` lives in a crate-private module, so no import path
    /// for it can link from an external doctest crate.
    ///
    /// ```ignore
    /// use db_handler::history::sensors::model::SensorTags;
    /// use db_handler::sensors::model::ProductType;
    ///
    /// let tags = SensorTags::new(
    ///     ProductType::ContactSensor,
    ///     123,
    ///     "AA:BB:CC:DD:EE:01".to_string(),
    /// )
    /// .expect("the MAC is well formed");
    /// let _ = tags;
    /// ```
    pub fn new(product_type: ProductType, sensor_id: u32, sensor_mac: String) -> Result<Self> {
        // Trust boundary: the MAC format is validated here AND at
        // deserialization. Public fields mean a constructor check alone
        // would be bypassable, but every construction path through this
        // constructor keeps the invariant.
        if !crate::common::helpers::mac::is_valid(&sensor_mac) {
            return Err(SensorHistoryError::invalid_format(format!(
                "Invalid MAC address format: {sensor_mac}"
            )));
        }
        Ok(Self {
            product_type,
            sensor_id,
            sensor_mac,
        })
    }
}
