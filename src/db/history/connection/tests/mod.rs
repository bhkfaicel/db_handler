//! Tests for the history client factory's timeout wiring.
//!
//! A history write against a server that accepts the connection but never
//! answers must fail through the client's request timeout instead of
//! hanging forever: before the timeouts existed, a hung or unreachable
//! InfluxDB stalled `set_history` indefinitely while the corresponding
//! SQLite update was already committed.

mod policy;

use crate::db::history::connection::{get_time_series_database_client, HistoryDbClient};
use anyhow::Result;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::{Duration, Instant};

/// Verifies the complete InfluxDB write request without contacting a real server.
#[tokio::test]
async fn write_sends_expected_influx_request() -> Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let server = std::thread::spawn(move || -> Result<(String, String, String)> {
        let (socket, _) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let mut headers = String::new();
        let mut content_length = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header)?;
            if header == "\r\n" {
                break;
            }
            headers.push_str(&header);
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse::<usize>()?;
                }
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body)?;
        let mut socket = reader.into_inner();
        socket.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")?;
        Ok((request_line, headers, String::from_utf8(body)?))
    });

    let client = HistoryDbClient::new(
        reqwest::Client::builder().http1_only().build()?,
        url::Url::parse(&format!("http://{addr}"))?,
        "home org".to_string(),
        "secret-token".to_string(),
    );
    client
        .write(
            "sensor bucket",
            "sensor_data,product_type=ContactSensor,is_open=true 123".to_string(),
        )
        .await?;

    let (request_line, headers, body) = server.join().expect("the request server must finish")?;
    assert!(request_line
        .starts_with("POST /api/v2/write?org=home+org&bucket=sensor+bucket&precision=ns HTTP/1.1"));
    assert!(headers
        .to_ascii_lowercase()
        .contains("authorization: bearer secret-token\r\n"));
    assert_eq!(
        body,
        "sensor_data,product_type=ContactSensor,is_open=true 123"
    );
    Ok(())
}

/// Verifies that non-successful InfluxDB responses become write errors.
#[tokio::test]
async fn write_rejects_http_error_status() -> Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let server = std::thread::spawn(move || -> Result<()> {
        let (mut socket, _) = listener.accept()?;
        socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n")?;
        Ok(())
    });

    let client = HistoryDbClient::new(
        reqwest::Client::builder().http1_only().build()?,
        url::Url::parse(&format!("http://{addr}"))?,
        "org".to_string(),
        "token".to_string(),
    );
    let result = client
        .write("bucket", "sensor_data value=1i 1".to_string())
        .await;

    assert!(result.is_err(), "HTTP 500 must fail the history write");
    server.join().expect("the error server must finish")?;
    Ok(())
}

/// Points the factory at an in-process server that accepts and never
/// writes, issues one minimal write with short timeouts (1 s connect /
/// 2 s request) and asserts the write fails within a bounded wall-clock
/// window instead of hanging.
///
/// `#[tokio::test]` on purpose: the factory and the client write are
/// async. The listener itself is a plain `std::net::TcpListener` served
/// by a plain OS thread — accepting and holding a socket needs no async
/// runtime features, and a blocking `accept` on the single-threaded test
/// runtime would deadlock against the client's own connect.
#[tokio::test]
async fn hung_influx_server_write_fails_through_request_timeout() -> Result<()> {
    // Ephemeral in-process listener: nothing here can reach a real network
    // service, so the test is hermetic and deterministic.
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;

    // Server side: accept exactly one connection (the write opens one) and
    // hold the socket open without ever writing a byte — the client's
    // request can then only end through its own timeout. Bounded hold so
    // the thread always exits; it is never joined by the test.
    std::thread::spawn(move || {
        let (_socket, _peer) = listener.accept().expect("the client must connect");
        std::thread::sleep(Duration::from_secs(30));
    });

    // Configuration pointing the history client at the silent server. The
    // primary URL is never exercised by this factory, so `:memory:` is fine.
    let config_path = std::env::temp_dir().join(format!(
        "dbh_history_timeout_cfg_{}.json",
        std::process::id()
    ));
    std::fs::write(
        &config_path,
        format!(
            r#"{{"primary_db_url":":memory:","history_db_url":"http://{addr}","history_db_org":"t","history_db_token":"t"}}"#
        ),
    )?;

    // `CONFIG_FILE` is process-global: take the shared test guard, which
    // serializes access across parallel tests and restores the previous
    // value on drop.
    let guard = crate::test_env::ConfigFileGuard::acquire();
    guard.set(Some(config_path.to_str().unwrap()));

    let (client, bucket) =
        get_time_series_database_client(Duration::from_secs(1), Duration::from_secs(2)).await?;

    let started = Instant::now();
    // Minimal line protocol: content is irrelevant, the request is cut off
    // by the timeout before any server-side processing.
    let write = client
        .write(&bucket, "timeout_probe value=1i 1".to_string())
        .await;
    let elapsed = started.elapsed();

    assert!(
        write.is_err(),
        "a write against a silently hanging server must fail, not succeed"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "the request timeout must end the wait quickly, took {elapsed:?}"
    );

    // Drop the guard: it restores the previous CONFIG_FILE value.
    drop(guard);
    let _ = std::fs::remove_file(&config_path);
    Ok(())
}
