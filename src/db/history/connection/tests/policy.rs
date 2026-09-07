//! Tests for the history client factory's transport policy.
//!
//! Plain `http://` carries the bearer token unencrypted (CWE-319), so the
//! factory must reject it against non-loopback hosts unless the
//! configuration explicitly opts in via `history_db_allow_insecure`.
//! `https://` and loopback `http://` are always accepted.

use anyhow::Result;

/// Builds a temporary configuration file containing the given history
/// URL and an optional `history_db_allow_insecure` flag, returns its path.
fn temp_config(history_db_url: &str, allow_insecure: Option<bool>) -> std::path::PathBuf {
    let insecure_line = match allow_insecure {
        Some(true) => r#","history_db_allow_insecure":true"#.to_string(),
        Some(false) => r#","history_db_allow_insecure":false"#.to_string(),
        // Field absent entirely: the serde default (false) must apply.
        None => String::new(),
    };
    // The file name must discriminate between flag variants: two otherwise
    // identical configs (same URL, different `history_db_allow_insecure`)
    // would otherwise share one file and overwrite each other mid-run.
    let path = std::env::temp_dir().join(format!(
        "dbh_history_policy_cfg_{}_{}_{}.json",
        std::process::id(),
        history_db_url
            .replace("://", "_")
            .replace([':', '/', '.'], "_"),
        allow_insecure
            .map(|b| b.to_string())
            .unwrap_or_else(|| "absent".into())
    ));
    std::fs::write(
        &path,
        format!(
            r#"{{"primary_db_url":":memory:","history_db_url":"{history_db_url}","history_db_org":"t","history_db_token":"t"{insecure_line}}}"#
        ),
    )
    .expect("temporary test config must be writable");
    path
}

/// Runs the factory against a fixed configuration and reports whether the
/// URL passed the transport policy (an error mentioning the policy is a
/// rejection; any other error propagates and fails the test).
async fn run_with_config(path: &std::path::Path) -> Result<()> {
    // `CONFIG_FILE` is process-global: take the shared test guard, which
    // serializes access across parallel tests and restores the previous
    // value on drop (whatever this helper's outcome).
    let guard = crate::test_env::ConfigFileGuard::acquire();
    guard.set(Some(path.to_str().unwrap()));
    let outcome = crate::db::history::connection::get_time_series_database_client(
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(2),
    )
    .await;
    drop(guard); // restore the previous CONFIG_FILE value
    outcome.map(|_| ())
}

#[tokio::test]
async fn http_non_loopback_without_opt_in_is_rejected() -> Result<()> {
    let path = temp_config("http://influx.example.com:8086", None);
    let err = run_with_config(&path)
        .await
        .expect_err("plain http against a non-loopback host must be rejected by default");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("https for non-loopback"),
        "rejection must state the transport policy, got: {msg}"
    );
    std::fs::remove_file(&path)?;
    Ok(())
}

#[tokio::test]
async fn http_non_loopback_with_opt_in_is_accepted() -> Result<()> {
    // The client is fully constructed (no network call happens here), so
    // an unreachable host only proves the policy let the URL through.
    let path = temp_config("http://influx.example.com:8086", Some(true));
    assert!(
        run_with_config(&path).await.is_ok(),
        "explicit history_db_allow_insecure must accept plain http"
    );
    std::fs::remove_file(&path)?;
    Ok(())
}

#[tokio::test]
async fn http_localhost_without_opt_in_is_accepted() -> Result<()> {
    let path = temp_config("http://localhost:8086", None);
    assert!(
        run_with_config(&path).await.is_ok(),
        "loopback hosts are always allowed, opt-in or not"
    );
    std::fs::remove_file(&path)?;
    Ok(())
}

#[tokio::test]
async fn https_is_accepted() -> Result<()> {
    let path = temp_config("https://influx.example.com", None);
    assert!(
        run_with_config(&path).await.is_ok(),
        "https is always allowed"
    );
    std::fs::remove_file(&path)?;
    Ok(())
}

#[tokio::test]
async fn malformed_url_is_rejected() -> Result<()> {
    let path = temp_config("not a url at all", None);
    let err = run_with_config(&path)
        .await
        .expect_err("an unparsable URL must be rejected with a clear error");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("not a valid URL"),
        "rejection must name the URL defect, got: {msg}"
    );
    std::fs::remove_file(&path)?;
    Ok(())
}
