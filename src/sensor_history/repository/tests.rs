#[cfg(test)]
mod tests {
    use crate::connection::get_time_series_database_client;
    use crate::sensor_history::model::{SensorDataPoint, SensorFields, SensorTags};
    use crate::sensors::model::ProductType;

    /// Test that a data point can be converted to line protocol.
    #[test]
    fn test_data_point_to_line_protocol() {
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

    /// Test that InfluxDB client can be created.
    #[tokio::test]
    async fn test_influx_client_creation() {
        let client = get_time_series_database_client().await;
        assert!(client.is_ok());
    }
}
