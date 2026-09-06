use crate::common::helpers::config::HomeIotConfig;
use std::{env, fs::File, io::Write};

#[test]
fn test_config_new_returns_error_when_file_not_found() {
    let nonexistent = env::temp_dir()
        .join("nonexistent_config_dir_12345")
        .join("config.json");
    // Environment variables are process-global: save and restore so this
    // test cannot poison every later test that needs CONFIG_FILE.
    let original = env::var("CONFIG_FILE").ok();
    env::set_var("CONFIG_FILE", nonexistent.to_str().unwrap());
    let result = HomeIotConfig::new();
    match original {
        Some(v) => env::set_var("CONFIG_FILE", v),
        None => {
            env::remove_var("CONFIG_FILE");
        }
    }
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to open config file")
            || err_msg.contains("No such file")
            || err_msg.contains("not found"),
        "expected file-not-found message, got: {}",
        err_msg
    );
}

#[test]
fn test_config_new_returns_error_when_json_invalid() {
    let tmp_dir = env::temp_dir();
    let path = tmp_dir.join("home_iot_config_invalid_test.json");
    let mut file = File::create(&path).expect("Failed to create temp config file");
    file.write_all(b"{ invalid json }")
        .expect("Failed to write");
    drop(file);

    let original = env::var("CONFIG_FILE").ok();
    env::set_var("CONFIG_FILE", path.to_str().unwrap());

    let result = HomeIotConfig::new();

    if let Some(v) = original {
        env::set_var("CONFIG_FILE", v);
    } else {
        env::remove_var("CONFIG_FILE");
    }
    let _ = std::fs::remove_file(&path);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid JSON") || err_msg.contains("invalid"),
        "expected invalid JSON message, got: {}",
        err_msg
    );
}

#[test]
fn test_home_iot_config_new_reads_from_config_file() {
    let tmp_dir = env::temp_dir();
    let path = tmp_dir.join("home_iot_config_test.json");

    {
        let mut file = File::create(&path).expect("Failed to create temp config file");
        let json = r#"{
            "primary_db_url": "sqlite:/tmp/home_iot.db",
            "history_db_url": "http://localhost:8086",
            "history_db_org": "test-org",
            "history_db_token": "test-token"
        }"#;
        file.write_all(json.as_bytes())
            .expect("Failed to write temp config");
    }

    let config = HomeIotConfig::load_from_path(path.to_str().unwrap())
        .expect("Valid config file should load");

    assert_eq!(config.primary_db_url(), "sqlite:/tmp/home_iot.db");
    assert_eq!(config.history_db_url(), "http://localhost:8086");
    assert_eq!(config.history_db_org(), "test-org");
    assert_eq!(config.history_db_token(), "test-token");

    let _ = std::fs::remove_file(&path);
}

/// A configuration file without `bucket_name` loads with the default
/// bucket: existing deployments and test configs stay valid untouched,
/// and the bucket is renamed through configuration, never through code.
#[test]
fn test_bucket_name_defaults_when_field_absent() {
    let tmp_dir = env::temp_dir();
    let path = tmp_dir.join("home_iot_config_bucket_default_test.json");

    {
        let mut file = File::create(&path).expect("Failed to create temp config file");
        // Deliberately no `bucket_name` field: the default must apply.
        let json = r#"{
            "primary_db_url": "sqlite:/tmp/home_iot.db",
            "history_db_url": "http://localhost:8086",
            "history_db_org": "test-org",
            "history_db_token": "test-token"
        }"#;
        file.write_all(json.as_bytes())
            .expect("Failed to write temp config");
    }

    let config = HomeIotConfig::load_from_path(path.to_str().unwrap())
        .expect("Valid config file should load");

    assert_eq!(config.bucket_name(), "test_bucket");

    let _ = std::fs::remove_file(&path);
}
