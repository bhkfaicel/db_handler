use crate::common::helpers::config::HomeIotConfig;
use anyhow::Result;
use influxdb2::Client;
use std::time::Duration;

/// Initializes and returns a time-series database client.
///
/// The configuration is read from the JSON file pointed to by the
/// `CONFIG_FILE` environment variable (falls back to `config.json` in the
/// current working directory); see [`HomeIotConfig`] for the exact format.
///
/// Currently returns an InfluxDB client, but the implementation can be
/// changed to support other time-series databases in the future.
///
/// # Configuration File Fields
///
/// * `history_db_url` - Database server URL (for example
///   "http://localhost:8086")
/// * `history_db_org` - Organization name
/// * `history_db_token` - Authentication token
/// * `bucket_name` - InfluxDB bucket that history writes target (optional,
///   defaults to "test_bucket" when absent)
///
/// # Returns
///
/// * `Ok((Client, bucket_name))` - Client created successfully together
///   with the configured InfluxDB bucket name that history writes target
/// * `Err(anyhow::Error)` - If configuration or client creation fails
///
/// # Arguments
///
/// * `connect_timeout` - Maximum wait for the TCP/TLS connection to the
///   InfluxDB server to establish
/// * `request_timeout` - Maximum wall-clock time for one complete HTTP
///   request/response exchange against the server
///
// Connection infrastructure must not depend on a domain error type: returning
// `anyhow::Result` decouples the `db` tree from the sensor history error
// module, which is the prerequisite for compiling `db` without the `sensors`
// feature.
pub(crate) async fn get_time_series_database_client(
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<(Client, String)> {
    // The configuration loader already reports failures as `anyhow::Error`;
    // propagating it preserves the original error chain (no string re-wrap).
    let config = HomeIotConfig::new()?;
    let url = config.history_db_url();
    let org = config.history_db_org();
    let token = config.history_db_token();
    // The bucket travels with the client through the construction chain
    // (`Connection` -> services -> repository) instead of a hardcoded
    // constant: deployments rename it in configuration, not in code.
    //
    // The client is built through `ClientBuilder::with_builder` so the
    // underlying reqwest client carries connect and request timeouts.
    // Without them a hung or unreachable InfluxDB server would stall
    // `set_history` indefinitely (the HTTP call never returns) while the
    // corresponding SQLite update is already committed, piling up sensor
    // events behind a stuck write. Connect failures surface within the
    // connect window; a server that accepts but never answers is cut off
    // by the overall request timeout.
    let client = influxdb2::ClientBuilder::with_builder(
        reqwest::ClientBuilder::new()
            .connect_timeout(connect_timeout)
            .timeout(request_timeout),
        url,
        org,
        token,
    )
    .build()?;
    Ok((client, config.bucket_name()))
}

#[cfg(test)]
mod tests;
