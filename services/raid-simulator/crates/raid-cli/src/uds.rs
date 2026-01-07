use std::time::Duration;

use anyhow::Context;
use http::Uri;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint};
use tower::util::service_fn;

pub async fn connect_uds(socket_path: &str, connect_timeout: Duration, rpc_timeout: Duration) -> anyhow::Result<Channel> {
    let socket_path = socket_path.to_owned();

    let endpoint = Endpoint::try_from("http://[::]:50051")
        .context("create tonic endpoint")?
        .connect_timeout(connect_timeout)
        .timeout(rpc_timeout);

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
