// Minimal behavioral tests for the user data models: constructor behavior
// and serde behavior, including the redaction of the secret password hash
// (Debug marker, serialization skip, deserialization kept).

use serde_json::json;

use crate::users::model::{InternalUser, User, UserSortKey};

#[test]
fn test_user_constructor_and_serde_round_trip() {
    // The constructor must store the representative values it is given.
    let user = User::new(1, "alice".to_string(), "alice@example.com".to_string());
    assert_eq!(user.id, 1);
    assert_eq!(user.username, "alice");
    assert_eq!(user.email, "alice@example.com");

    // Serde round-trip: serialize -> deserialize -> equality.
    let json = serde_json::to_string(&user).expect("serialization must succeed");
    let back: User = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(back, user);
}

#[test]
fn test_internal_user_constructor_serde_and_redaction() {
    // The crate-internal model additionally carries the optional hash.
    let internal = InternalUser::new(
        2,
        "bob".to_string(),
        "bob@example.com".to_string(),
        Some("hash".to_string()),
    );
    assert_eq!(internal.id, 2);
    assert_eq!(internal.password_hash.as_deref(), Some("hash"));

    // Debug output replaces the hash value with the fixed redaction
    // marker; only the hash presence stays visible for debugging.
    let debug = format!("{:?}", internal);
    assert!(
        !debug.contains("Some(\"hash\")"),
        "Debug output leaked the password hash: {}",
        debug
    );
    assert!(
        debug.contains("[REDACTED]"),
        "Debug output must show the redaction marker: {}",
        debug
    );

    // Serialization skips the hash field entirely, so no serialized form
    // of this model ever carries the secret.
    let serialized = serde_json::to_string(&internal).expect("serialization must succeed");
    assert!(
        !serialized.contains("hash"),
        "serialized form leaked the password hash: {}",
        serialized
    );

    // Deserialization is kept: a complete payload carrying the hash
    // still parses back into the model with every field in place.
    let back: InternalUser = serde_json::from_value(json!({
        "id": 2,
        "username": "bob",
        "email": "bob@example.com",
        "password_hash": "hash",
    }))
    .expect("deserialization of a complete payload must succeed");
    assert_eq!(back, internal);
}

#[test]
fn test_user_sort_key_serde_round_trip_and_rendering() {
    // Each variant round-trips through its lowercase wire format and the
    // hand-written renderings must match the repository column names.
    for (key, name) in [
        (UserSortKey::Id, "id"),
        (UserSortKey::UserName, "username"),
        (UserSortKey::Email, "email"),
    ] {
        let json = serde_json::to_string(&key).expect("serialization must succeed");
        assert_eq!(json, format!("\"{name}\""));
        let back: UserSortKey = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(back.as_ref(), name);
    }
}
