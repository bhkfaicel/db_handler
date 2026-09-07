// Behavioral tests for the sensor data model's MAC field: deserialization
// is a trust boundary for the MAC format, so a hostile payload must be
// refused outright and a well-formed lowercase address must still parse.

use crate::sensors::model::{Sensor, Status};

/// Deserialization is a trust boundary for the MAC field: the fields are
/// public, so a payload carrying a malformed MAC (here, line-protocol
/// metacharacters) must be refused outright.
#[test]
fn test_sensor_deserialization_rejects_invalid_mac() {
    let payload = serde_json::json!({
        "id": 3,
        "mac": "AA,BB CC=DD:EE:FF",
        "status": "InProgress",
        "sensor_type": null,
    });
    let err = serde_json::from_value::<Sensor>(payload)
        .expect_err("an invalid MAC must fail deserialization");
    assert!(
        err.to_string().contains("Invalid MAC address format"),
        "expected an invalid-MAC deserialization error, got: {}",
        err
    );
}

/// The MAC format is case-insensitive: a well-formed lowercase address
/// deserializes unchanged.
#[test]
fn test_sensor_deserialization_accepts_lowercase_mac() {
    let payload = serde_json::json!({
        "id": 3,
        "mac": "aa:bb:cc:dd:ee:01",
        "status": "InProgress",
        "sensor_type": null,
    });
    let back =
        serde_json::from_value::<Sensor>(payload).expect("a well-formed MAC must deserialize");
    assert_eq!(back.mac, "aa:bb:cc:dd:ee:01");
    assert_eq!(back.status, Status::InProgress);
}
