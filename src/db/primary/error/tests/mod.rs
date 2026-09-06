//! Unit tests for the SQLite unique-violation message extraction.
//!
//! SQLite reports the violated unique constraint only through the error
//! message text, so the extraction function is the single decoder of that
//! message. These fixtures pin its behavior on real-world message shapes:
//! a well-formed report, a non-unique constraint report, and a malformed
//! unique-violation report.

use super::extract_unique_constraint;

#[test]
fn test_extracts_constraint_from_well_formed_message() {
    // Real SQLite message shape for a single-column unique constraint
    // (column-level UNIQUE on keys.mac).
    let message = "UNIQUE constraint failed: keys.mac";
    assert_eq!(
        extract_unique_constraint(message),
        Some("keys.mac".to_string())
    );
}

#[test]
fn test_extracts_composite_constraint_column_list() {
    // Real SQLite message shape for a composite unique constraint
    // (UNIQUE (value, mac)): the full column list is the constraint identity.
    let message = "UNIQUE constraint failed: nonces.value, nonces.mac";
    assert_eq!(
        extract_unique_constraint(message),
        Some("nonces.value, nonces.mac".to_string())
    );
}

#[test]
fn test_returns_none_for_non_unique_constraint_message() {
    // A constraint failure that is NOT a unique violation must not match:
    // the sqlx kind check happens before extraction, so this message can
    // only reach the function if the message and kind disagree.
    let message = "NOT NULL constraint failed: keys.value";
    assert_eq!(extract_unique_constraint(message), None);
}

#[test]
fn test_returns_none_for_malformed_message_without_constraint() {
    // Malformed report: the unique-violation prefix is present but SQLite
    // named no constraint after it (padded and bare variants).
    assert_eq!(extract_unique_constraint("UNIQUE constraint failed:"), None);
    assert_eq!(
        extract_unique_constraint("UNIQUE constraint failed:   "),
        None
    );
}

#[test]
fn test_returns_none_for_unrelated_message() {
    // Any message without the SQLite unique-violation prefix yields None.
    assert_eq!(extract_unique_constraint("database is locked"), None);
    assert_eq!(extract_unique_constraint(""), None);
}
