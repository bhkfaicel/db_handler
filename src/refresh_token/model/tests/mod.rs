// Minimal behavioral test for the refresh token data model: construction
// from representative values and serde round-trip stability
// (serialize -> deserialize -> equality).

use chrono::{TimeZone, Utc};

use crate::refresh_token::model::RefreshToken;

#[test]
fn test_refresh_token_serde_round_trip() {
    // Representative token: owner, secret value, creation and expiry
    // timestamps, and the used flag.
    let token = RefreshToken {
        id: 4,
        user_id: 2,
        token: "token-value".to_string(),
        expire_at: Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap(),
        created_at: Utc.with_ymd_and_hms(2026, 9, 6, 12, 0, 0).unwrap(),
        is_used: false,
    };

    // Serde round-trip: serialize -> deserialize -> equality.
    let json = serde_json::to_string(&token).expect("serialization must succeed");
    let back: RefreshToken = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(back, token);
}
