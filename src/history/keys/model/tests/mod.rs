// Minimal behavioral test for the historical key data model: construction
// from representative values and serde round-trip stability
// (serialize -> deserialize -> equality).

use chrono::{TimeZone, Utc};

use crate::history::keys::model::HistoricalKey;

#[test]
fn test_historical_key_serde_round_trip() {
    // Representative historical key: fixed 32-byte value and UTC expiry.
    let key = HistoricalKey {
        id: 11,
        value: vec![0xCD; 32],
        expire_at: Utc.with_ymd_and_hms(2031, 6, 5, 4, 3, 2).unwrap(),
    };

    // Serde round-trip: serialize -> deserialize -> equality.
    let json = serde_json::to_string(&key).expect("serialization must succeed");
    let back: HistoricalKey = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(back, key);
}
