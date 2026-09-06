// Minimal behavioral test for the nonce data model: construction from
// representative values and serde round-trip stability
// (serialize -> deserialize -> equality).

use crate::nonces::model::Nonce;

#[test]
fn test_nonce_serde_round_trip() {
    // Representative nonce: fixed 12-byte value bound to a sensor MAC.
    let nonce = Nonce {
        id: 9,
        value: vec![0x1F; 12],
        mac: "AA:BB:CC:DD:EE:02".to_string(),
    };

    // Serde round-trip: serialize -> deserialize -> equality.
    let json = serde_json::to_string(&nonce).expect("serialization must succeed");
    let back: Nonce = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(back, nonce);
}
