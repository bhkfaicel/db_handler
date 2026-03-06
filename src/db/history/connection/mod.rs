use crate::common::helpers::config::HomeIotConfig;
use crate::history::sensors::error::{Result, SensorHistoryError};
use influxdb2::Client;

/// Initializes and returns a time-series database client.
///
/// Reads the database configuration from environment variables.
/// Supports configuration via environment variables with automatic
/// loading from `.env` or `.env.local` files.
///
/// Currently returns an InfluxDB client, but the implementation can be
/// changed to support other time-series databases in the future.
///
/// # Environment Variables
///
/// The function first tries to read generic variable names, then falls back
/// to InfluxDB-specific names for backward compatibility:
///
/// * `TIMESERIES_DB_URL` - Database server URL (default: "http://localhost:8086")
/// * `TIMESERIES_DB_ORG` - Organization name (required)
/// * `TIMESERIES_DB_TOKEN` - Authentication token (required)
///
/// # Returns
///
/// * `Ok(TimeSeriesClient)` - Client created successfully
/// * `Err(SensorHistoryError)` - If configuration or client creation fails
///
/// # Example
///
/// no_run
/// use db_handler::history::sensors::connection::get_time_series_database_client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = get_time_series_database_client().await?;
/// # Ok(())
/// # }
///
pub(crate) async fn get_time_series_database_client() -> Result<Client> {
    let config = HomeIotConfig::new()
        .map_err(|e| SensorHistoryError::configuration(e.to_string()))?;
    let url = config.history_db_url();
    let org = config.history_db_org();
    let token = config.history_db_token();
    Ok(Client::new(url, org, token))
}
