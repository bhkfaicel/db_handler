// Behavioral test for the key data model: the raw key bytes are secret
// key material and must never appear in Debug output or in the serialized
// form, while deserialization of a complete payload stays available.

use chrono::{TimeZone, Utc};
use serde_json::json;

use crate::keys::model::Key;

#[test]
fn test_key_secret_never_leaks_through_debug_or_serialization() {
    // Representative key: fixed 32-byte value and a UTC expiry timestamp.
    let key = Key {
        id: 7,
        value: vec![0xAB; 32],
        expire_at: Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap(),
        mac: "AA:BB:CC:DD:EE:01".to_string(),
    };
    // The secret bytes render as the decimal 171; a run of four cannot
    // occur in any other field's rendering.
    let debug_leak = "171, 171, 171, 171";
    let json_leak = "171,171,171,171";

    // Debug output replaces the key bytes with the fixed redaction
    // marker; every other field keeps its normal rendering.
    let debug = format!("{:?}", key);
    assert!(
        !debug.contains(debug_leak),
        "Debug output leaked key bytes: {}",
        debug
    );
    assert!(
        debug.contains("[REDACTED]"),
        "Debug output must show the redaction marker: {}",
        debug
    );

    // Serialization skips the value field entirely, so no serialized
    // form of this model ever carries the key bytes.
    let serialized = serde_json::to_string(&key).expect("serialization must succeed");
    assert!(
        !serialized.contains(json_leak),
        "serialized form leaked key bytes: {}",
        serialized
    );
    assert!(
        !serialized.contains("\"value\""),
        "serialization must skip the value field: {}",
        serialized
    );

    // Deserialization is kept: a complete payload carrying the key bytes
    // still parses back into the model.
    let back: Key = serde_json::from_value(json!({
        "id": 7,
        "value": [171, 171],
        "expire_at": "2030-01-02T03:04:05Z",
        "mac": "AA:BB:CC:DD:EE:01",
    }))
    .expect("deserialization of a complete payload must succeed");
    assert_eq!(back.value, vec![0xAB; 2]);
    assert_eq!(back.id, 7);
    assert_eq!(back.mac, "AA:BB:CC:DD:EE:01");
}

/// Deserialization is a trust boundary for the MAC field: the fields are
/// public, so a payload carrying a malformed MAC (here, line-protocol
/// metacharacters) must be refused outright.
#[test]
fn test_key_deserialization_rejects_invalid_mac() {
    let payload = json!({
        "id": 7,
        "value": [171, 171],
        "expire_at": "2030-01-02T03:04:05Z",
        "mac": "AA,BB CC=DD:EE:FF",
    });
    let err = serde_json::from_value::<Key>(payload)
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
fn test_key_deserialization_accepts_lowercase_mac() {
    let payload = json!({
        "id": 7,
        "value": [171, 171],
        "expire_at": "2030-01-02T03:04:05Z",
        "mac": "aa:bb:cc:dd:ee:01",
    });
    let back = serde_json::from_value::<Key>(payload)
        .expect("a well-formed lowercase MAC must deserialize");
    assert_eq!(back.mac, "aa:bb:cc:dd:ee:01");
}
