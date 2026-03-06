use crate::history::sensors::error::{map_history_error_to_anyhow, SensorHistoryError};

/// Tests for error handling and conversion in sensor history operations.
#[test]
fn test_sensor_history_error_write() {
    let error = SensorHistoryError::write("Failed to write data");
    assert!(error.to_string().contains("Failed to write data"));
}

#[test]
fn test_sensor_history_error_connection() {
    let error = SensorHistoryError::connection("Connection failed");
    assert!(error.to_string().contains("Connection failed"));
}

#[test]
fn test_sensor_history_error_query() {
    let error = SensorHistoryError::query("Query failed");
    assert!(error.to_string().contains("Query failed"));
}

#[test]
fn test_sensor_history_error_invalid_format() {
    let error = SensorHistoryError::invalid_format("Invalid format");
    assert!(error.to_string().contains("Invalid format"));
}

#[test]
fn test_sensor_history_error_configuration() {
    let error = SensorHistoryError::configuration("Config error");
    assert!(error.to_string().contains("Config error"));
}

#[test]
fn test_sensor_history_error_not_found() {
    let error = SensorHistoryError::not_found(123);
    assert!(error.to_string().contains("123"));
}

#[test]
fn test_sensor_history_error_operation() {
    let error = SensorHistoryError::operation("Operation failed");
    assert!(error.to_string().contains("Operation failed"));
}

#[test]
fn test_map_history_error_to_anyhow() {
    let error = SensorHistoryError::write("Test error");
    let anyhow_error = map_history_error_to_anyhow(error);
    assert!(anyhow_error.to_string().contains("Test error"));
}

#[test]
fn test_sensor_history_error_debug() {
    let error = SensorHistoryError::write("Debug test");
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Write"));
}
