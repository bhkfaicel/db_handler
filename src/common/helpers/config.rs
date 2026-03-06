use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::from_reader;
use std::{env, fs::File, io::BufReader};

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
///   "history_db_token": "my-secret-token"
/// }
/// ```
///
/// Only the fields defined in this struct are read from the JSON file.
#[derive(Debug, Deserialize)]
pub struct HomeIotConfig {
    primary_db_url: String,
    history_db_url: String,
    history_db_org: String,
    history_db_token: String,
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
}
