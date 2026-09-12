//! Tests for the contact-sensor history repository: the constructor
//! contract and the InfluxDB data-point conversion.
//!
//! These are pure unit tests: the constructor only stores the shared
//! client and the configured bucket name, and the conversion function
//! builds InfluxDB line protocol in memory; neither ever touches the
//! network.

use std::sync::Arc;

use crate::db::history::connection::HistoryDbClient;

use super::{escape_key, escape_string, render_line_protocol, Repository};
use crate::history::sensors::error::SensorHistoryError;
use crate::history::sensors::model::fields::{SensorFields, TimeUnit, VibrationCount};
use crate::history::sensors::model::point::SensorDataPoint;
use crate::history::sensors::model::tags::SensorTags;
use crate::sensors::model::ProductType;
use chrono::{DateTime, Utc};

/// Builds a contact data point with the given point timestamp.
fn contact_point(timestamp: DateTime<Utc>) -> SensorDataPoint {
    let tags = SensorTags::new(
        ProductType::ContactSensor,
        1,
        "AA:BB:CC:DD:EE:FF".to_string(),
    )
    .expect("a well-formed MAC must be accepted");
    let fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: None,
    };
    SensorDataPoint {
        measurement: "sensor_data".to_string(),
        tags,
        fields,
        timestamp,
    }
}

/// Asserts the error is `InvalidFormat` and mentions the timestamp, proving
/// the failure is reported instead of falling back to epoch 0.
fn assert_invalid_format(error: SensorHistoryError) {
    match error {
        SensorHistoryError::InvalidFormat(msg) => {
            assert!(
                msg.contains("timestamp"),
                "Error should mention the timestamp, got: {msg}"
            );
        }
        other => panic!("Expected InvalidFormat, got {:?}", other),
    }
}

/// A point timestamp beyond the i64 nanosecond range (past year ~2262) must
/// yield an `InvalidFormat` error instead of silently writing epoch 0.
#[test]
fn invalid_point_timestamp_yields_invalid_format() {
    // ~1e10 seconds after the epoch exceeds the i64 nanosecond range, so
    // `timestamp_nanos_opt()` returns `None` for it.
    let out_of_range = DateTime::from_timestamp(10_000_000_000, 0).unwrap();
    let error = render_line_protocol(&contact_point(out_of_range)).unwrap_err();
    assert_invalid_format(error);
}

/// A sensor-side timestamp beyond the i64 nanosecond range must yield an
/// `InvalidFormat` error instead of silently writing epoch 0.
#[test]
fn invalid_sensor_timestamp_yields_invalid_format() {
    let mut point = contact_point(Utc::now());
    point.fields = SensorFields::Contact {
        is_open: true,
        vibration_count: None,
        sensor_timestamp: Some(DateTime::from_timestamp(10_000_000_000, 0).unwrap()),
    };
    let error = render_line_protocol(&point).unwrap_err();
    assert_invalid_format(error);
}

/// Asserts the exact line protocol for a minimal contact point.
#[test]
fn renders_minimal_contact_point() {
    let timestamp = DateTime::from_timestamp(1, 2).expect("static timestamp must be valid");
    let line = render_line_protocol(&contact_point(timestamp)).expect("point must render");

    assert_eq!(
        line,
        "sensor_data,product_type=ContactSensor,sensor_id=1,sensor_mac=AA:BB:CC:DD:EE:FF is_open=true 1000000002"
    );
}

/// Asserts the exact line protocol for every optional contact field.
#[test]
fn renders_optional_contact_fields_and_sensor_timestamp() {
    let mut point = contact_point(DateTime::from_timestamp(1, 2).unwrap());
    point.fields = SensorFields::Contact {
        is_open: false,
        vibration_count: Some(VibrationCount {
            count: 2,
            time_window: 3,
            unit: TimeUnit::Seconds,
        }),
        sensor_timestamp: Some(DateTime::from_timestamp(4, 5).unwrap()),
    };

    let line = render_line_protocol(&point).expect("point must render");

    assert_eq!(
        line,
        "sensor_data,product_type=ContactSensor,sensor_id=1,sensor_mac=AA:BB:CC:DD:EE:FF is_open=false,vibration_count=2i,vibration_time_window=3i,vibration_time_unit=\"seconds\",sensor_timestamp=4000000005i 1000000002"
    );
}

/// Asserts escaping for measurement/tag keys and quoted string fields.
#[test]
fn escapes_line_protocol_keys_and_strings() {
    assert_eq!(
        escape_key(r"measurement, key=value"),
        r"measurement\,\ key\=value"
    );
    assert_eq!(escape_string(r#"seconds\"value"#), r#"seconds\\\"value"#);
}

/// The constructor stores the provided shared client and the provided
/// bucket name in the new repository instance: the bucket is whatever the
/// configuration supplied, never a constant decided in code.
#[test]
fn new_returns_repository_initialized_with_client_and_bucket() {
    // Build a client against a local URL: no connection is attempted.
    let client = Arc::new(HistoryDbClient::new(
        reqwest::Client::new(),
        url::Url::parse("http://localhost:8086").expect("static URL must parse"),
        "org".to_string(),
        "token".to_string(),
    ));
    let bucket = "configured-sensor-bucket".to_string();
    let repository = Repository::new(Arc::clone(&client), bucket.clone());
    // The instance must reference the exact client and bucket it was given.
    assert!(Arc::ptr_eq(&client, &repository.client));
    assert_eq!(repository.bucket, bucket);
}
