//! Shared test support for the battery test module.

use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

// Spawns a one-shot local HTTP fake bound on an ephemeral port: the returned
// address is plugged into a `HistoryDbClient` by the caller, while the
// spawned thread accepts a single connection, reads the request up to the
// body (parsing `Content-Length` from the headers), replies with the
// configured `status` code, and resolves to the received body so tests can
// assert on exactly what was written.
pub(super) fn history_fake(status: u16) -> (std::net::SocketAddr, std::thread::JoinHandle<String>) {
    // Ephemeral port keeps the tests hermetic and collision-free.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(socket);
        let mut length = 0;
        // Read the head until the blank separator line, tracking the
        // announced body length.
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        // Read exactly the announced body, answer, and hand the body back.
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        write!(
            reader.get_mut(),
            "HTTP/1.1 {status} Result\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        String::from_utf8(body).unwrap()
    });
    (addr, server)
}
