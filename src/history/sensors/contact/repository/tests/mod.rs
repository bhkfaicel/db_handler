//! Tests for the contact-sensor history repository: the constructor
//! contract and the InfluxDB data-point conversion.
//!
//! These are pure unit tests: the constructor only stores the shared
//! client and the configured bucket name, and the conversion function
//! builds InfluxDB line protocol in memory; neither ever touches the
//! network.

use std::sync::Arc;

use crate::db::history::connection::HistoryDbClient;

use super::{render_line_protocol, Repository};
use crate::history::sensors::error::SensorHistoryError;
use crate::history::sensors::model::fields::SensorFields;
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
