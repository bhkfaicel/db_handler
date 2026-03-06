#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        connection::get_time_series_database_client, sensor_history::service::SensorHistoryService,
    };

    /// Test that service can be created with a valid client.
    #[tokio::test]
    async fn test_service_creation() {
        let client = get_time_series_database_client().await;
        assert!(client.is_ok());
        let _service = SensorHistoryService::new(Arc::new(client.unwrap()));
        // Service created successfully
        assert!(true);
    }
}
