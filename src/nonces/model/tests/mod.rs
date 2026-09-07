// Minimal behavioral tests for the nonce data model: construction from
// representative values, serde round-trip stability
// (serialize -> deserialize -> equality), and MAC format enforcement at
// the deserialization trust boundary.

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

/// Deserialization is a trust boundary for the MAC field: the fields are
/// public, so a payload carrying a malformed MAC (here, line-protocol
/// metacharacters) must be refused outright.
#[test]
fn test_nonce_deserialization_rejects_invalid_mac() {
    let payload = serde_json::json!({
        "id": 9,
        "value": [31, 31],
        "mac": "AA,BB CC=DD:EE:FF",
    });
    let err = serde_json::from_value::<Nonce>(payload)
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
fn test_nonce_deserialization_accepts_lowercase_mac() {
    let payload = serde_json::json!({
        "id": 9,
        "value": [31, 31],
        "mac": "aa:bb:cc:dd:ee:02",
    });
    let back = serde_json::from_value::<Nonce>(payload)
        .expect("a well-formed lowercase MAC must deserialize");
    assert_eq!(back.mac, "aa:bb:cc:dd:ee:02");
}
