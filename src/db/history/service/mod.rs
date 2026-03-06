use influxdb2::Client;
use std::sync::Arc;
pub trait HistoryDbService: Sized {
    /// Creates a new instance of the service with a InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - InfluxDB client
    ///
    /// # Returns
    ///
    /// A new instance of the service initialized with the provided client.
    fn new(client: Arc<Client>) -> Self;
}
