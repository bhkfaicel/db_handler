use crate::sensor_history::model::fields::TimeUnit;
use crate::sensor_history::model::fields::{SensorFields, VibrationCount};
use crate::sensor_history::model::point::SensorDataPoint;
use crate::sensor_history::model::tags::SensorTags;
use crate::sensors::model::ProductType;

#[test]
fn test_contact_fields_to_line_protocol() {
    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
    };
    assert_eq!(fields.to_line_protocol(), "is_open=true");

    let fields = SensorFields::Contact {
        is_open: false,
        vibration_count: None,
    };
    assert_eq!(fields.to_line_protocol(), "is_open=false");

    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: Some(VibrationCount {
            count: 5,
            time_window: 10,
            unit: TimeUnit::Seconds,
        }),
    };
    let line_protocol = fields.to_line_protocol();
    assert!(line_protocol.contains("is_open=true"));
    assert!(line_protocol.contains("vibration_count=5"));
    assert!(line_protocol.contains("vibration_time_window=10"));
    assert!(line_protocol.contains("vibration_time_unit=\"seconds\""));
}

#[test]
fn test_temperature_fields_to_line_protocol() {
    let fields = SensorFields::Temperature {
        temperature: 22.5,
        humidity: Some(45.0),
    };
    let line_protocol = fields.to_line_protocol();
    assert!(line_protocol.contains("temperature=22.5"));
    assert!(line_protocol.contains("humidity=45.0"));

    let fields = SensorFields::Temperature {
        temperature: 20.0,
        humidity: None,
    };
    let line_protocol = fields.to_line_protocol();
    assert_eq!(line_protocol, "temperature=20.0");
}

#[test]
fn test_vibration_fields_to_line_protocol() {
    let fields = SensorFields::Vibration {
        vibration_level: 0.5,
        frequency: 50.0,
    };
    let line_protocol = fields.to_line_protocol();
    assert!(line_protocol.contains("vibration_level=0.5"));
    assert!(line_protocol.contains("frequency=50.0"));
}

#[test]
fn test_numeric_fields_to_line_protocol() {
    let fields = SensorFields::Numeric { value: 42.5 };
    assert_eq!(fields.to_line_protocol(), "value=42.5");
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
    };

    let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

    let line_protocol = point.to_line_protocol();
    assert!(line_protocol.starts_with("sensor_data,"));
    assert!(line_protocol.contains("is_open=true"));
    assert!(line_protocol.contains("product_type=ContactSensor"));
    assert!(line_protocol.contains("sensor_id=123"));
}
