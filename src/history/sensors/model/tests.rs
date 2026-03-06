use chrono::Utc;

use crate::history::sensors::model::fields::TimeUnit;
use crate::history::sensors::model::fields::{SensorFields, VibrationCount};
use crate::history::sensors::model::point::SensorDataPoint;
use crate::history::sensors::model::tags::SensorTags;
use crate::sensors::model::ProductType;

#[test]
fn test_contact_fields_to_line_protocol() {
    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: None,
    };
    assert_eq!(fields.to_line_protocol(), "is_open=true");

    let fields = SensorFields::Contact {
        is_open: false,
        vibration_count: None,
        sensor_timestamp: None,
    };
    assert_eq!(fields.to_line_protocol(), "is_open=false");

    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: Some(VibrationCount {
            count: 5,
            time_window: 10,
            unit: TimeUnit::Seconds,
        }),
        sensor_timestamp: Some(Utc::now()),
    };
    let line_protocol = fields.to_line_protocol();
    assert!(line_protocol.contains("is_open=true"));
    assert!(line_protocol.contains("vibration_count=5"));
    assert!(line_protocol.contains("vibration_time_window=10"));
    assert!(line_protocol.contains("vibration_time_unit=\"seconds\""));
}

#[test]
fn test_sensor_tags_new() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    );

    assert_eq!(tags.product_type, ProductType::ContactSensor);
    assert_eq!(tags.sensor_id, 123);
    assert_eq!(tags.sensor_mac, "AA:BB:CC:DD:EE:01");
}

#[test]
fn test_sensor_tags_to_line_protocol() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    );

    let line_protocol = tags.to_line_protocol();
    assert!(line_protocol.contains("product_type=ContactSensor"));
    assert!(line_protocol.contains("sensor_id=123"));
    assert!(line_protocol.contains("sensor_mac=AA\\:BB\\:CC\\:DD\\:EE\\:01"));
}

#[test]
fn test_sensor_data_point_new() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    );

    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: None,
    };

    let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

    assert_eq!(point.measurement, "sensor_data");
    assert_eq!(point.tags.sensor_id, 123);
}

#[test]
fn test_sensor_data_point_to_line_protocol() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    );

    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: None,
    };

    let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

    let line_protocol = point.to_line_protocol();
    assert!(line_protocol.starts_with("sensor_data,"));
    assert!(line_protocol.contains("is_open=true"));
    assert!(line_protocol.contains("product_type=ContactSensor"));
    assert!(line_protocol.contains("sensor_id=123"));
}
