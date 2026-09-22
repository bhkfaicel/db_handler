//! Battery reading model.

use chrono::{DateTime, Utc};

/// Latest battery state of a sensor: resting and loaded voltage.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BatteryReading {
    /// Identifier of the sensor the reading belongs to.
    pub sensor_id: u32,
    /// Battery voltage in millivolts at rest.
    pub resting_mv: u16,
    /// Battery voltage in millivolts under load.
    pub loaded_mv: u16,
    /// Time the reading was received by the server.
    pub received_at: DateTime<Utc>,
}
