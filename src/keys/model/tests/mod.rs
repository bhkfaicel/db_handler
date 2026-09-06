// Minimal behavioral test for the key data model: construction from
// representative values and serde round-trip stability
// (serialize -> deserialize -> equality).

use chrono::{TimeZone, Utc};

use crate::keys::model::Key;

#[test]
fn test_key_serde_round_trip() {
    // Representative key: fixed 32-byte value and a UTC expiry timestamp.
    let key = Key {
        id: 7,
        value: vec![0xAB; 32],
        expire_at: Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap(),
        mac: "AA:BB:CC:DD:EE:01".to_string(),
    };

    // Serde round-trip: serialize -> deserialize -> equality.
    let json = serde_json::to_string(&key).expect("serialization must succeed");
    let back: Key = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(back, key);
}
