use crate::history::sensors::model::point::SensorDataPoint;
use crate::history::sensors::model::tags::SensorTags;
use crate::sensors::model::ProductType;

#[test]
fn test_sensor_tags_new() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    )
    .expect("a well-formed MAC must be accepted");

    assert_eq!(tags.product_type, ProductType::ContactSensor);
    assert_eq!(tags.sensor_id, 123);
    assert_eq!(tags.sensor_mac, "AA:BB:CC:DD:EE:01");
}

/// The constructor is a trust boundary: a MAC carrying line-protocol
/// metacharacters (`,` `=` space) and a structurally broken address
/// (five pairs) must both be rejected as an invalid format, never
/// rendered into a time-series tag.
#[test]
fn test_sensor_tags_rejects_invalid_mac_at_construction() {
    // Line-protocol metacharacters that a tag value must never carry.
    let malicious = "AA,BB CC=DD:EE:FF";
    let err = SensorTags::new(ProductType::ContactSensor, 123, malicious.to_string())
        .expect_err("a MAC with line-protocol metacharacters must be rejected");
    assert!(
        err.to_string().contains("Invalid MAC address format"),
        "expected an invalid-format error, got: {}",
        err
    );

    // Structurally wrong: five pairs instead of six.
    let short = "AA:BB:CC:DD:EE";
    assert!(
        SensorTags::new(ProductType::ContactSensor, 123, short.to_string()).is_err(),
        "a five-pair MAC must be rejected"
    );
}

/// The format is case-insensitive: lowercase hexadecimal pairs are
/// accepted unchanged.
#[test]
fn test_sensor_tags_accepts_lowercase_mac() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "aa:bb:cc:dd:ee:01".to_string(),
    )
    .expect("lowercase hexadecimal pairs must be accepted");
    assert_eq!(tags.sensor_mac, "aa:bb:cc:dd:ee:01");
}

/// Deserialization enforces the very same format rule as the
/// constructor: the fields are public, so the derived `Deserialize`
/// would otherwise bypass every construction-side check.
#[test]
fn test_sensor_tags_rejects_invalid_mac_at_deserialization() {
    // Malicious payload: the MAC carries line-protocol metacharacters.
    let payload = serde_json::json!({
        "product_type": "ContactSensor",
        "sensor_id": 123,
        "sensor_mac": "AA,BB CC=DD:EE:FF",
    });
    let err = serde_json::from_value::<SensorTags>(payload)
        .expect_err("an invalid MAC must fail deserialization");
    assert!(
        err.to_string().contains("Invalid MAC address format"),
        "expected an invalid-MAC deserialization error, got: {}",
        err
    );

    // A well-formed lowercase MAC still parses (case-insensitive rule).
    let valid = serde_json::json!({
        "product_type": "ContactSensor",
        "sensor_id": 123,
        "sensor_mac": "aa:bb:cc:dd:ee:01",
    });
    let tags =
        serde_json::from_value::<SensorTags>(valid).expect("a well-formed MAC must deserialize");
    assert_eq!(tags.sensor_mac, "aa:bb:cc:dd:ee:01");
}

#[test]
fn test_sensor_data_point_new() {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        123,
        "AA:BB:CC:DD:EE:01".to_string(),
    )
    .expect("a well-formed MAC must be accepted");

    let fields = crate::history::sensors::model::SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: None,
    };

    let point = SensorDataPoint::new("sensor_data".to_string(), tags, fields);

    assert_eq!(point.measurement, "sensor_data");
    assert_eq!(point.tags.sensor_id, 123);
}
