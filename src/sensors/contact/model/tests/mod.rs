// Minimal behavioral test for the contact sensor model: construction from
// a wrapped sensor plus accessor behavior (this type has no serde derive,
// so the observable surface is the constructor and its accessors).

use chrono::NaiveDate;

use super::ContactSensor;
use crate::sensors::model::{Sensor, Status, TypedSensorTrait};

#[test]
fn test_contact_sensor_constructor_and_accessors() {
    // Build the wrapped sensor and the contact view over it.
    let sensor = Sensor::new(3, "AA:BB:CC:DD:EE:03".to_string(), Status::Installed);
    let updated_at = NaiveDate::from_ymd_opt(2026, 9, 6)
        .expect("valid date")
        .and_hms_opt(12, 0, 0)
        .expect("valid time");
    let contact = ContactSensor::new(sensor, Some(true), Some(updated_at));

    // The typed accessors expose the wrapped sensor identity unchanged.
    assert_eq!(contact.id(), 3);
    assert_eq!(contact.mac(), "AA:BB:CC:DD:EE:03");
    assert_eq!(contact.status(), &Status::Installed);
    // Contact-specific state round-trips through its accessors.
    assert_eq!(contact.is_open(), Some(true));
    assert_eq!(contact.updated_at(), Some(updated_at));
}
