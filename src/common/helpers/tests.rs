use crate::common::helpers::{
    config::HomeIotConfig,
    controller::{hash_password, verify_password},
};
use std::{env, fs::File, io::Write};

#[test]
fn test_config_new_returns_error_when_file_not_found() {
    let nonexistent = env::temp_dir().join("nonexistent_config_dir_12345").join("config.json");
    env::set_var("CONFIG_FILE", nonexistent.to_str().unwrap());
    let result = HomeIotConfig::new();
    let _ = env::remove_var("CONFIG_FILE");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to open config file") || err_msg.contains("No such file") || err_msg.contains("not found"),
        "expected file-not-found message, got: {}",
        err_msg
    );
}

#[test]
fn test_config_new_returns_error_when_json_invalid() {
    let tmp_dir = env::temp_dir();
    let path = tmp_dir.join("home_iot_config_invalid_test.json");
    let mut file = File::create(&path).expect("Failed to create temp config file");
    file.write_all(b"{ invalid json }").expect("Failed to write");
    drop(file);

    let original = env::var("CONFIG_FILE").ok();
    env::set_var("CONFIG_FILE", path.to_str().unwrap());

    let result = HomeIotConfig::new();

    if let Some(v) = original {
        env::set_var("CONFIG_FILE", v);
    } else {
        let _ = env::remove_var("CONFIG_FILE");
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
fn test_hash_password() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
}

#[test]
fn test_verify_password_incorrect() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(!verify_password("wrong_password", &hashed).unwrap());
}

#[test]
fn test_verify_password_correct() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
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
        file.write_all(json.as_bytes()).expect("Failed to write temp config");
    }

    let config = HomeIotConfig::load_from_path(path.to_str().unwrap()).expect("Valid config file should load");

    assert_eq!(config.primary_db_url(), "sqlite:/tmp/home_iot.db");
    assert_eq!(config.history_db_url(), "http://localhost:8086");
    assert_eq!(config.history_db_org(), "test-org");
    assert_eq!(config.history_db_token(), "test-token");

    let _ = std::fs::remove_file(&path);
}
