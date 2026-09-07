// Behavioral test for the historical key data model: the raw key bytes
// are secret key material and must never appear in Debug output or in the
// serialized form, while deserialization of a complete payload stays
// available.

use chrono::{TimeZone, Utc};
use serde_json::json;

use crate::history::keys::model::HistoricalKey;

#[test]
fn test_historical_key_secret_never_leaks_through_debug_or_serialization() {
    // Representative historical key: fixed 32-byte value and UTC expiry.
    let key = HistoricalKey {
        id: 11,
        value: vec![0xCD; 32],
        expire_at: Utc.with_ymd_and_hms(2031, 6, 5, 4, 3, 2).unwrap(),
    };
    // The secret bytes render as the decimal 205; a run of four cannot
    // occur in any other field's rendering.
    let debug_leak = "205, 205, 205, 205";
    let json_leak = "205,205,205,205";

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
    let back: HistoricalKey = serde_json::from_value(json!({
        "id": 11,
        "value": [205, 205],
        "expire_at": "2031-06-05T04:03:02Z",
    }))
    .expect("deserialization of a complete payload must succeed");
    assert_eq!(back.value, vec![0xCD; 2]);
    assert_eq!(back.id, 11);
}
