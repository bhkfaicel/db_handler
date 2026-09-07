use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::from_reader;
use std::{env, fmt, fs::File, io::BufReader};

/// Global configuration for the Home IoT backend.
///
/// The configuration is loaded from a JSON file whose path is provided by
/// the `CONFIG_FILE` environment variable. If `CONFIG_FILE` is not set,
/// the code falls back to `config.json` in the current working directory.
///
/// # Expected JSON structure
///
/// ```json
/// {
///   "primary_db_url": "sqlite:/path/to/home_iot.db",
///   "history_db_url": "http://localhost:8086",
///   "history_db_org": "my-org",
///   "history_db_token": "my-secret-token",
///   "bucket_name": "my-bucket"
/// }
/// ```
///
/// `bucket_name` is optional: when the field is absent the
/// [`default_bucket_name`] value applies, so existing configuration files
/// stay valid without any edit.
///
/// Only the fields defined in this struct are read from the JSON file.
#[derive(Deserialize)]
pub struct HomeIotConfig {
    primary_db_url: String,
    history_db_url: String,
    history_db_org: String,
    /// Authentication token for the time-series database. A credential:
    /// serialization skips it, so the secret can never travel through a
    /// serialized form of this struct.
    #[serde(skip_serializing)]
    history_db_token: String,
    /// Name of the InfluxDB bucket that history writes target.
    #[serde(default = "default_bucket_name")]
    bucket_name: String,
    /// Explicit opt-in to plain `http://` against a non-loopback history
    /// database host (for example a Docker-internal `influxdb` hostname or
    /// a private-LAN gateway). Defaults to false: the history client then
    /// only accepts `https://`, or plain `http://` for loopback hosts.
    /// Traffic stays unencrypted on the wire when this is true (CWE-319),
    /// so it must only be set for networks where that is understood.
    #[serde(default)]
    history_db_allow_insecure: bool,
}

// Hand-written Debug: the history token is a credential, so it renders as
// a fixed marker instead of its value and no log line, panic message or
// debug dump can ever leak it. Every other field keeps its normal
// readable rendering for operators.
impl fmt::Debug for HomeIotConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HomeIotConfig")
            .field("primary_db_url", &self.primary_db_url)
            .field("history_db_url", &self.history_db_url)
            .field("history_db_org", &self.history_db_org)
            .field("history_db_token", &"[REDACTED]")
            .field("bucket_name", &self.bucket_name)
            .field("history_db_allow_insecure", &self.history_db_allow_insecure)
            .finish()
    }
}

/// Default InfluxDB bucket used when a configuration file omits
/// `bucket_name`: pre-existing deployments keep working unchanged and the
/// value can be renamed per deployment through the configuration file
/// instead of in code.
fn default_bucket_name() -> String {
    "test_bucket".to_string()
}

impl HomeIotConfig {
    /// Loads the configuration from the JSON file pointed to by `CONFIG_FILE`.
    ///
    /// If `CONFIG_FILE` is not set, falls back to `config.json` in the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or the JSON cannot be
    /// deserialized into `HomeIotConfig`. The caller can then display a clear
    /// message or log before exiting.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::env;
    /// use std::fs::File;
    /// use std::io::Write;
    /// use db_handler::common::helpers::HomeIotConfig;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let path = std::env::temp_dir().join("home_iot_config_example.json");
    /// let mut file = File::create(&path)?;
    /// file.write_all(br#"{"primary_db_url":"sqlite:/tmp/db","history_db_url":"http://localhost:8086","history_db_org":"org","history_db_token":"token"}"#)?;
    /// env::set_var("CONFIG_FILE", path.to_str().unwrap());
    /// let config = HomeIotConfig::new()?;
    /// assert_eq!(config.primary_db_url(), "sqlite:/tmp/db");
    /// # let _ = std::fs::remove_file(&path);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        let config_file = env::var("CONFIG_FILE").unwrap_or_else(|_| "config.json".to_string());
        Self::load_from_path(&config_file)
    }

    /// Loads configuration from a specific path. Used by `new()` and by tests to avoid mutating `CONFIG_FILE`.
    pub(crate) fn load_from_path(config_file: &str) -> Result<Self> {
        let file = File::open(config_file)
            .with_context(|| format!("Failed to open config file: {}", config_file))?;
        let reader = BufReader::new(file);
        let config: HomeIotConfig = from_reader(reader)
            .with_context(|| format!("Invalid JSON in config file: {}", config_file))?;
        Ok(config)
    }

    /// Returns the primary (SQLite) database URL.
    pub fn primary_db_url(&self) -> String {
        self.primary_db_url.clone()
    }

    /// Returns the time-series/history database URL.
    pub fn history_db_url(&self) -> String {
        self.history_db_url.clone()
    }

    /// Returns the organization for the time-series/history database.
    pub fn history_db_org(&self) -> String {
        self.history_db_org.clone()
    }

    /// Returns the authentication token for the time-series/history database.
    pub fn history_db_token(&self) -> String {
        self.history_db_token.clone()
    }

    /// Returns the InfluxDB bucket name that history writes target.
    pub fn bucket_name(&self) -> String {
        self.bucket_name.clone()
    }

    /// Returns whether plain `http://` is explicitly allowed against a
    /// non-loopback history database host.
    pub fn history_db_allow_insecure(&self) -> bool {
        self.history_db_allow_insecure
    }
}
