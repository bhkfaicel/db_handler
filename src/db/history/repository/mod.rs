use influxdb2::Client;
use std::sync::Arc;

pub(crate) trait HistoryDbRepository: Sized {
    /// Creates a new instance of the repository with a InfluxDB client.
    ///
    /// # Arguments
    ///
    /// * `client` - InfluxDB client
    ///
    /// # Returns
    ///
    /// A new instance of the repository initialized with the provided client.
    fn new(client: Arc<Client>) -> Self;
}

#[cfg(test)]
mod tests;
