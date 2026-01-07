use std::time::Duration;

use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, info, warn};

use crate::pb::metrics as pb;
use crate::uds::connect_uds;

pub struct SenderConfig {
    pub socket_path: String,
    pub connect_timeout: Duration,
    pub rpc_timeout: Duration,

    pub backoff_initial: Duration,
    pub backoff_max: Duration,
    pub jitter_ratio: f64,

    pub conn_buffer: usize,
    pub shutdown_grace: Duration,

    pub auth_token: Option<String>,
}

pub struct SenderStats {
    pub dropped_batches: u64,
    pub reconnects: u64,
    pub send_errors: u64,
}

pub async fn run_sender(
    mut rx: mpsc::Receiver<pb::MetricsBatch>,
    mut shutdown: watch::Receiver<bool>,
    cfg: SenderConfig,
) -> SenderStats {
    let mut stats = SenderStats {
        dropped_batches: 0,
        reconnects: 0,
        send_errors: 0,
    };

    let mut rng = StdRng::from_os_rng();
    let mut backoff = cfg.backoff_initial;

    let auth_md: Option<MetadataValue<_>> = match cfg
        .auth_token
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        Some(tok) => match MetadataValue::try_from(tok) {
            Ok(v) => Some(v),
            Err(e) => {
                panic!("METRICS_AUTH_TOKEN is not valid metadata value: {e}");
            }
        },
        None => None,
    };

    loop {
        if *shutdown.borrow() {
            info!("sender: shutdown requested");
            break;
        }

        info!("sender: connecting via UDS: {}", cfg.socket_path);

        let channel = match connect_uds(&cfg.socket_path, cfg.connect_timeout, cfg.rpc_timeout).await {
            Ok(ch) => {
                backoff = cfg.backoff_initial;
                ch
            }
            Err(err) => {
                stats.reconnects += 1;
                let sleep_dur = with_jitter(backoff, cfg.jitter_ratio, &mut rng);
                warn!("sender: connect failed: {err:#}; retry in {:?}", sleep_dur);

                tokio::select! {
                    _ = tokio::time::sleep(sleep_dur) => {},
                    _ = shutdown.changed() => {},
                }

                backoff = bump_backoff(backoff, cfg.backoff_max);
                continue;
            }
        };

        let mut client = pb::metrics_ingestor_client::MetricsIngestorClient::new(channel);

        let (conn_tx, conn_rx) = mpsc::channel::<pb::MetricsBatch>(cfg.conn_buffer);
        let outbound = ReceiverStream::new(conn_rx);

        let mut req = Request::new(outbound);
        if let Some(tok) = auth_md.clone() {
            req.metadata_mut().insert("x-metrics-token", tok);
        }

        let mut push_handle = tokio::spawn(async move { client.push(req).await });

        info!("sender: stream opened");

        let mut push_result: Option<
            Result<Result<tonic::Response<pb::PushResponse>, tonic::Status>, tokio::task::JoinError>,
        > = None;

        let mut conn_tx = conn_tx;
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("sender: shutdown -> closing stream");
                        break;
                    }
                },

                maybe_batch = rx.recv() => {
                    match maybe_batch {
                        Some(batch) => {
                            if let Err(_e) = conn_tx.send(batch).await {
                                stats.send_errors += 1;
                                warn!("sender: stream send failed (conn closed) -> reconnect");
                                break;
                            }
                        }
                        None => {
                            info!("sender: input channel closed -> closing stream");
                            break;
                        }
                    }
                },

                res = &mut push_handle => {
                    push_result = Some(res);
                    break;
                },
            }
        }

        drop(conn_tx);

        if push_result.is_none() {
            match tokio::time::timeout(cfg.shutdown_grace, &mut push_handle).await {
                Ok(res) => push_result = Some(res),
                Err(_timeout) => {
                    warn!("sender: shutdown grace timeout; exiting");
                }
            }
        }

        if let Some(res) = push_result {
            match res {
                Ok(Ok(resp)) => {
                    let r = resp.into_inner();
                    debug!(
                        "sender: push response: accepted_batches={}, accepted_samples={}, rejected_samples={}",
                        r.accepted_batches, r.accepted_samples, r.rejected_samples
                    );
                }
                Ok(Err(status)) => {
                    stats.send_errors += 1;
                    warn!("sender: push() ended with gRPC status: {status}");
                }
                Err(join_err) => {
                    stats.send_errors += 1;
                    warn!("sender: push task join error: {join_err}");
                }
            }
        }

        if *shutdown.borrow() {
            break;
        }

        stats.reconnects += 1;
        let sleep_dur = with_jitter(backoff, cfg.jitter_ratio, &mut rng);
        warn!("sender: reconnecting in {:?}", sleep_dur);

        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {},
            _ = shutdown.changed() => {},
        }

        backoff = bump_backoff(backoff, cfg.backoff_max);
    }

    while let Ok(_batch) = rx.try_recv() {
        stats.dropped_batches += 1;
    }

    stats
}

fn bump_backoff(cur: Duration, max: Duration) -> Duration {
    let next_ms = (cur.as_millis() as u64).saturating_mul(2);
    let next = Duration::from_millis(next_ms.max(1));
    if next > max { max } else { next }
}

fn with_jitter(base: Duration, ratio: f64, rng: &mut StdRng) -> Duration {
    if ratio <= 0.0 {
        return base;
    }
    let base_ms = base.as_millis() as u64;
    let extra = ((base_ms as f64) * ratio * rng.random::<f64>()) as u64;
    Duration::from_millis(base_ms.saturating_add(extra))
}
