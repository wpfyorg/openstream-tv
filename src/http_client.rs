//! Construction of the shared HTTP client.
//!
//! The same client serves provider metadata, authentication, EPG fetches and
//! media segments. With no limits, a peer that accepts the connection and then
//! stops sending pins the request for the life of the process — for EPG that
//! also pins one of the twenty concurrency permits, so twenty such connections
//! stop the guide from ever regenerating again.
//!
//! The two bounds below are deliberately different in kind:
//!
//! * `CONNECT_TIMEOUT` covers the TCP/TLS handshake, where no data is expected.
//! * `READ_IDLE_TIMEOUT` applies to each read, so it bounds silence rather than
//!   duration: a segment or a long live body that keeps delivering data is
//!   never cut, while a peer that stops mid-response fails in bounded time.
//!
//! Metadata and authentication calls additionally carry `METADATA_TIMEOUT` as
//! an overall deadline, because a response that trickles one byte at a time
//! never idles long enough for the read timeout to fire.

use std::time::Duration;

/// Time allowed for the TCP (and TLS) handshake.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Longest idle gap tolerated on an established connection.
pub const READ_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Overall deadline for a request that returns a small document (login,
/// token refresh, plan summary) rather than a media stream.
pub const METADATA_TIMEOUT: Duration = Duration::from_secs(20);

/// The client builder every HTTP user in this binary starts from.
pub fn builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_IDLE_TIMEOUT)
}

pub fn client() -> reqwest::Result<reqwest::Client> {
    builder().build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    async fn serve_once(handler: impl FnOnce(tokio::net::TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handler(socket);
        });
        format!("http://{addr}/metadata")
    }

    /// A peer that accepts the connection and never answers must not hold the
    /// request open indefinitely.
    #[tokio::test]
    async fn a_stalled_response_times_out() {
        let url = serve_once(|socket| {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(30)).await;
                drop(socket);
            });
        })
        .await;
        let client = builder()
            .read_timeout(Duration::from_millis(400))
            .build()
            .unwrap();

        let started = Instant::now();
        let err = client.get(&url).send().await.unwrap_err();
        assert!(err.is_timeout(), "expected a timeout, got {err}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "took {:?}",
            started.elapsed()
        );
    }

    /// Headers can arrive and the body then stall: reading the body must fail
    /// in bounded time rather than returning partial content as a success.
    #[tokio::test]
    async fn a_stalled_body_fails_the_read() {
        let url = serve_once(|mut socket| {
            tokio::spawn(async move {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 64\r\n\r\n")
                    .await
                    .unwrap();
                socket.write_all(b"partial").await.unwrap();
                tokio::time::sleep(Duration::from_secs(30)).await;
            });
        })
        .await;
        let client = builder()
            .read_timeout(Duration::from_millis(400))
            .build()
            .unwrap();

        let response = client.get(&url).send().await.unwrap();
        let err = response.text().await.unwrap_err();
        assert!(err.is_timeout(), "expected a timeout, got {err}");
    }

    /// The idle timeout bounds silence, not total duration: a body that keeps
    /// producing data for longer than the timeout must still complete.
    #[tokio::test]
    async fn a_steady_stream_survives_the_idle_timeout() {
        let url = serve_once(|mut socket| {
            tokio::spawn(async move {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n")
                    .await
                    .unwrap();
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    socket.write_all(b"x").await.unwrap();
                }
            });
        })
        .await;
        let client = builder()
            .read_timeout(Duration::from_millis(400))
            .build()
            .unwrap();

        let body = client.get(&url).send().await.unwrap().text().await.unwrap();
        assert_eq!(body, "x".repeat(10));
    }

    /// A body that errors mid-transfer surfaces the failure instead of reading
    /// as a short successful response.
    #[tokio::test]
    async fn a_dropped_connection_is_an_error_not_a_short_body() {
        let url = serve_once(|mut socket| {
            tokio::spawn(async move {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 64\r\n\r\npartial")
                    .await
                    .unwrap();
                drop(socket);
            });
        })
        .await;
        let client = builder().build().unwrap();

        let response = client.get(&url).send().await.unwrap();
        assert!(response.text().await.is_err());
    }
}
