mod pb;
mod sender;
mod simulator;
mod uds;

use std::time::Duration;

use clap::Parser;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::pb::metrics;
use crate::sender::{run_sender, SenderConfig};
use crate::simulator::SyntheticSimulator;

#[derive(Parser, Debug)]
#[command(name = "raid-cli", about = "RAID simulator metrics streamer (UDS gRPC)")]
struct Args {
    #[arg(long, env = "METRICS_SOCKET_PATH", default_value = "/sockets/metrics-gateway.sock")]
    socket_path: String,

    #[arg(long, env = "METRICS_SOURCE_ID", default_value = "raid-simulator")]
    source_id: String,

    #[arg(long, env = "METRICS_INTERVAL_MS", default_value_t = 1000)]
    interval_ms: u64,

    #[arg(long, env = "METRICS_OPS_PER_TICK", default_value_t = 200)]
    ops_per_tick: u32,

    #[arg(long, env = "METRICS_QUEUE_CAP", default_value_t = 2048)]
    queue_cap: usize,

    #[arg(long, env = "METRICS_CONN_BUFFER", default_value_t = 512)]
    conn_buffer: usize,

    #[arg(long, env = "METRICS_CONNECT_TIMEOUT_MS", default_value_t = 2000)]
    connect_timeout_ms: u64,

    #[arg(long, env = "METRICS_RPC_TIMEOUT_MS", default_value_t = 5000)]
    rpc_timeout_ms: u64,

    #[arg(long, env = "METRICS_BACKOFF_INITIAL_MS", default_value_t = 250)]
    backoff_initial_ms: u64,

    #[arg(long, env = "METRICS_BACKOFF_MAX_MS", default_value_t = 10_000)]
    backoff_max_ms: u64,

    #[arg(long, env = "METRICS_JITTER_RATIO", default_value_t = 0.2)]
    jitter_ratio: f64,

    #[arg(long, env = "METRICS_SHUTDOWN_GRACE_MS", default_value_t = 1500)]
    shutdown_grace_ms: u64,

    #[arg(long, env = "METRICS_AUTH_TOKEN", default_value = "")]
    auth_token: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let args = Args::parse();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let (tx, rx) = mpsc::channel::<metrics::MetricsBatch>(args.queue_cap);

    let generator = tokio::spawn(run_generator(
        tx,
        shutdown_rx.clone(),
        args.source_id.clone(),
        Duration::from_millis(args.interval_ms),
        args.ops_per_tick,
    ));

    let auth_token = args.auth_token.trim().to_string();
    let auth_token = if auth_token.is_empty() { None } else { Some(auth_token) };

    let sender_cfg = SenderConfig {
        socket_path: args.socket_path.clone(),
        connect_timeout: Duration::from_millis(args.connect_timeout_ms),
        rpc_timeout: Duration::from_millis(args.rpc_timeout_ms),
        backoff_initial: Duration::from_millis(args.backoff_initial_ms),
        backoff_max: Duration::from_millis(args.backoff_max_ms),
        jitter_ratio: args.jitter_ratio,
        conn_buffer: args.conn_buffer,
        shutdown_grace: Duration::from_millis(args.shutdown_grace_ms),
        auth_token,
    };

    let sender = tokio::spawn(run_sender(rx, shutdown_rx.clone(), sender_cfg));

    #[cfg(unix)]
    {
        let sigterm_fut = sigterm();
        tokio::pin!(sigterm_fut);

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown: ctrl-c");
            },
            _ = &mut sigterm_fut => {
                info!("shutdown: SIGTERM");
            },
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        info!("shutdown: ctrl-c");
    }

    let _ = shutdown_tx.send(true);

    let _ = generator.await;
    let stats = sender.await?;

    info!(
        "exit: reconnects={}, send_errors={}, dropped_batches={}",
        stats.reconnects, stats.send_errors, stats.dropped_batches
    );

    Ok(())
}

async fn run_generator(
    tx: mpsc::Sender<metrics::MetricsBatch>,
    mut shutdown: watch::Receiver<bool>,
    source_id: String,
    interval: Duration,
    ops_per_tick: u32,
) {
    let disk_ids = vec!["disk0", "disk1", "disk2", "disk3"]
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let raid_ids = vec!["raid0", "raid1", "raid3"]
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut sim = SyntheticSimulator::new(disk_ids, raid_ids);

    let mut seq_no: u64 = 1;
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut dropped: u64 = 0;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let batch = sim.next_batch(&source_id, seq_no, ops_per_tick);
                seq_no = seq_no.wrapping_add(1);

                match tx.try_send(batch) {
                    Ok(_) => {}
                    Err(_e) => {
                        dropped += 1;
                        if dropped % 100 == 0 {
                            warn!("generator: dropped_batches={}", dropped);
                        }
                    }
                }
            },
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("generator: shutdown");
                    break;
                }
            },
        }
    }
}

#[cfg(unix)]
async fn sigterm() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut s = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    s.recv().await;
}
