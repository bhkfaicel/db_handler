use crate::sensors::model::{Sensor, Status, TypedSensorTrait};
use chrono::NaiveDateTime;
/// Represents a contact sensor with history capabilities.
///
/// This type wraps a `Sensor` and provides convenient methods for
/// recording historical data specific to contact sensors.
#[derive(Clone)]
pub struct ContactSensor {
    /// The underlying sensor data
    id: u32,
    /// MAC address of the sensor.
    mac: String,
    /// Current status of the sensor.
    status: Status,
    /// Whether the sensor is open.
    is_open: Option<bool>,
    /// Timestamp of the last update.
    updated_at: Option<NaiveDateTime>,
}

impl ContactSensor {
    /// Creates a new `ContactSensor` from a `Sensor`.
    ///
    /// # Arguments
    ///
    /// * `sensor` - The sensor data
    ///
    /// # Example
    ///
    ///no_run
    /// # // Note: sensors::contact::model is private, so this example uses no_run
    /// # use crate::sensors::contact::ContactSensor;
    /// # use crate::sensors::model::{Sensor, Status};
    /// #
    /// # let sensor = Sensor::new(1, "AA:BB:CC:DD:EE:01".to_string(), Status::Installed);
    /// # let contact_sensor = ContactSensor::new(sensor);
    ///
    pub(crate) fn new(
        sensor: Sensor,
        is_open: Option<bool>,
        updated_at: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id: sensor.id,
            mac: sensor.mac,
            status: sensor.status,
            is_open,
            updated_at,
        }
    }
    pub fn is_open(&self) -> Option<bool> {
        self.is_open
    }
    pub fn updated_at(&self) -> Option<NaiveDateTime> {
        self.updated_at
    }
}

impl std::fmt::Debug for ContactSensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContactSensor")
            .field("id", &self.id)
            .field("mac", &self.mac)
            .field("status", &self.status)
            .field("is_open", &self.is_open)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl TypedSensorTrait for ContactSensor {
    fn id(&self) -> u32 {
        self.id
    }

    fn mac(&self) -> &str {
        &self.mac
    }
    fn status(&self) -> &Status {
        &self.status
    }
}

// Behavioral tests for the contact sensor model live in their own
// directory next to the source file, per the project test layout rule.
#[cfg(test)]
mod tests;
