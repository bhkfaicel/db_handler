// Minimal behavioral tests for the user data models: constructor behavior
// and serde round-trip stability (serialize -> deserialize -> equality).

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
fn test_internal_user_constructor_and_serde_round_trip() {
    // The crate-internal model additionally carries the optional hash.
    let internal = InternalUser::new(
        2,
        "bob".to_string(),
        "bob@example.com".to_string(),
        Some("hash".to_string()),
    );
    assert_eq!(internal.id, 2);
    assert_eq!(internal.password_hash.as_deref(), Some("hash"));

    // Serde round-trip keeps every field, the hash included, in place.
    let json = serde_json::to_string(&internal).expect("serialization must succeed");
    let back: InternalUser = serde_json::from_str(&json).expect("deserialization must succeed");
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
