// Behavioral test for the refresh token data model: the secret token
// value must never appear in Debug output or in the serialized form,
// while deserialization of a complete payload stays available.

use chrono::{TimeZone, Utc};
use serde_json::json;

use crate::refresh_token::model::RefreshToken;

#[test]
fn test_refresh_token_secret_never_leaks_through_debug_or_serialization() {
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

    // Debug output replaces the secret value with the fixed redaction
    // marker; every other field keeps its normal rendering.
    let debug = format!("{:?}", token);
    assert!(
        !debug.contains("token-value"),
        "Debug output leaked the token value: {}",
        debug
    );
    assert!(
        debug.contains("[REDACTED]"),
        "Debug output must show the redaction marker: {}",
        debug
    );

    // Serialization skips the token field entirely, so no serialized
    // form of this model ever carries the secret.
    let serialized = serde_json::to_string(&token).expect("serialization must succeed");
    assert!(
        !serialized.contains("token-value"),
        "serialized form leaked the token value: {}",
        serialized
    );
    assert!(
        !serialized.contains("\"token\""),
        "serialization must skip the token field: {}",
        serialized
    );

    // Deserialization is kept: a complete payload carrying the token
    // still parses back into the model with every field in place.
    let back: RefreshToken = serde_json::from_value(json!({
        "id": 4,
        "user_id": 2,
        "token": "token-value",
        "expire_at": "2030-01-02T03:04:05Z",
        "created_at": "2026-09-06T12:00:00Z",
        "is_used": false,
    }))
    .expect("deserialization of a complete payload must succeed");
    assert_eq!(back, token);
}
