//! Tests for the users-domain password helpers, relocated with the code
//! they cover.

use super::{hash_password, verify_password};

#[test]
fn test_hash_password() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
}

#[test]
fn test_verify_password_incorrect() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(!verify_password("wrong_password", &hashed).unwrap());
}

#[test]
fn test_verify_password_correct() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
}
