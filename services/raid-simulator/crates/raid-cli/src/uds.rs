//! Unix-domain socket helpers for connecting to the metrics gateway.

use std::time::Duration;

use anyhow::Context;
use http::Uri;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint};
use tower::util::service_fn;

/// `connect_uds` connects to a gRPC endpoint over a Unix domain socket.
///
/// # Arguments
/// * `socket_path` - Path to the Unix domain socket.
/// * `connect_timeout` - Timeout for establishing the connection.
/// * `rpc_timeout` - Optional per-RPC timeout.
///
/// # Returns
/// A configured gRPC channel.
///
/// # Errors
/// Returns an error if the connection cannot be established.
pub async fn connect_uds(
    socket_path: &str,
    connect_timeout: Duration,
    rpc_timeout: Option<Duration>,
) -> anyhow::Result<Channel> {
    let socket_path = socket_path.to_owned();

    let mut endpoint = Endpoint::try_from("http://[::]:50051")
        .context("create tonic endpoint")?
        .connect_timeout(connect_timeout);

    if let Some(t) = rpc_timeout {
        endpoint = endpoint.timeout(t);
    }

    let channel = endpoint
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = socket_path.clone();
            async move {
                let stream = UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await
        .context("connect to UDS")?;

    Ok(channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_uds_errors_for_missing_socket() {
        let err = connect_uds(
            "/tmp/raid-cli-missing.sock",
            Duration::from_millis(10),
            None,
        )
        .await
        .expect_err("expected error");
        let msg = format!("{err:#}");
        assert!(msg.contains("connect to UDS"));
    }
}
