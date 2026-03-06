use crate::sensors::model::Status;
use std::str::FromStr;
#[test]
fn from_str() {
    assert_eq!(Status::from_str("InProgress").unwrap(), Status::InProgress);
    assert_eq!(Status::from_str("Installed").unwrap(), Status::Installed);
    assert_eq!(Status::from_str("OnUpdate").unwrap(), Status::OnUpdate);
    assert!(Status::from_str("Unknown").is_err());
}

#[test]
fn display() {
    assert_eq!(Status::InProgress.to_string(), "InProgress");
    assert_eq!(Status::Installed.to_string(), "Installed");
    assert_eq!(Status::OnUpdate.to_string(), "OnUpdate");
}
