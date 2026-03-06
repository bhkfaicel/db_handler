use crate::sensors::model::ProductType;
use std::str::FromStr;
#[test]
fn from_str() {
    assert_eq!(
        ProductType::from_str("ContactSensor").unwrap(),
        ProductType::ContactSensor
    );
    assert!(ProductType::from_str("Unknown").is_err());
}

#[test]
fn display() {
    assert_eq!(ProductType::ContactSensor.to_string(), "ContactSensor");
}
