use crate::sensors::model::ProductType;
use serde::{Deserialize, Serialize};

/// Tags for time-series sensor data points.
///
/// Tags are indexed metadata used for filtering and grouping data.
/// In time-series databases, tags should contain values that are frequently queried
/// but don't change frequently.
///
/// # Example
///
/// no_run
/// # // Note: sensor_history is pub(crate), so this example uses no_run
/// # use crate::sensor_history::model::SensorTags;
/// # use crate::sensors::model::ProductType;
/// #
/// # let tags = SensorTags {
/// #     product_type: ProductType::ContactSensor,
/// #     sensor_id: 123,
/// #     sensor_mac: "AA:BB:CC:DD:EE:01".to_string(),
/// # };
///
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensorTags {
    /// Type of product/sensor (e.g., ContactSensor, TemperatureSensor)
    pub product_type: ProductType,
    /// Unique sensor identifier
    pub sensor_id: u32,
    /// Sensor MAC address
    pub sensor_mac: String,
}

impl SensorTags {
    /// Creates a new `SensorTags` instance.
    ///
    /// # Arguments
    ///
    /// * `product_type` - Type of the sensor product
    /// * `sensor_id` - Unique sensor identifier
    /// * `sensor_mac` - Sensor MAC address
    ///
    /// # Returns
    ///
    /// A new `SensorTags` instance.
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::sensor_history::model::SensorTags;
    /// # use crate::sensors::model::ProductType;
    /// #
    /// # let tags = SensorTags::new(
    /// #     ProductType::ContactSensor,
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// # );
    ///
    pub fn new(product_type: ProductType, sensor_id: u32, sensor_mac: String) -> Self {
        Self {
            product_type,
            sensor_id,
            sensor_mac,
        }
    }

    /// Converts tags to Line Protocol format (compatible with InfluxDB).
    ///
    /// # Returns
    ///
    /// A string in the format: `product_type=ContactSensor,sensor_id=123,sensor_mac=AA:BB:CC:DD:EE:01`
    ///
    /// # Example
    ///
    /// no_run
    /// # // Note: sensor_history is pub(crate), so this example uses no_run
    /// # use crate::sensor_history::model::SensorTags;
    /// # use crate::sensors::model::ProductType;
    /// #
    /// # let tags = SensorTags::new(
    /// #     ProductType::ContactSensor,
    /// #     123,
    /// #     "AA:BB:CC:DD:EE:01".to_string(),
    /// # );
    /// # let line_protocol = tags.to_line_protocol();
    /// # assert!(line_protocol.contains("product_type=ContactSensor"));
    /// # assert!(line_protocol.contains("sensor_id=123"));
    ///
    pub fn to_line_protocol(&self) -> String {
        format!(
            "product_type={},sensor_id={},sensor_mac={}",
            self.product_type,
            self.sensor_id,
            self.sensor_mac.replace(':', "\\:")
        )
    }
}
