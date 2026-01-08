use anyhow::Result;
use clap::Parser;

mod cli;
pub mod fs;
mod mount;

mod pb;
mod sender;
mod simulator;
mod uds;

use cli::{Cli, Command, RaidMode};
use fs::DEFAULT_CHUNK_SIZE;
use mount::run_fuse;

use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use crate::pb::metrics;
use crate::sender::{SenderConfig, SenderStats, run_sender};
use crate::simulator::SyntheticSimulator;

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Command::Fuse(args) => run_fuse_with_synthetic_metrics(args),
        Command::Metrics(args) => run_metrics_only(args),
    }
}

fn init_tracing() {
    if tracing::dispatcher::has_been_set() {
        return;
    }

    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();
}

fn run_fuse_with_synthetic_metrics(args: cli::FuseArgs) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let metrics_args = args.metrics.clone();
    let metrics_thread = start_metrics_thread(metrics_args, shutdown_rx);

    let fuse_res = run_fuse_command(args);

    let _ = shutdown_tx.send(true);

    match metrics_thread.join() {
        Ok(Ok(stats)) => {
            info!(
                "metrics: exit: reconnects={}, send_errors={}, dropped_batches={}",
                stats.reconnects, stats.send_errors, stats.dropped_batches
            );
        }
        Ok(Err(e)) => {
            warn!("metrics: exited with error: {:#}", e);
        }
        Err(_panic) => {
            warn!("metrics: background thread panicked");
        }
    }

    fuse_res
}

fn run_fuse_command(args: cli::FuseArgs) -> Result<()> {
    let cli::FuseArgs {
        mount_point,
        disk_dir,
        raid,
        disks,
        disk_size,
        metrics: _,
    } = args;

    let disk_size = disk_size.max(1);

    match (raid, disks) {
        (RaidMode::Raid0, 1) => {
            run_fuse::<1, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
        }
        (_, 1) => Err(anyhow::anyhow!("raid mode requires at least 2 disks")),
        (_, 2) => run_fuse::<2, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 3) => run_fuse::<3, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 4) => run_fuse::<4, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 5) => run_fuse::<5, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 6) => run_fuse::<6, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 7) => run_fuse::<7, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        (_, 8) => run_fuse::<8, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size),
        _ => Err(anyhow::anyhow!(
            "unsupported disk count {disks}; supported range is 1-8"
        )),
    }
}

fn run_metrics_only(args: cli::MetricsArgs) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let metrics_task = tokio::spawn(run_metrics_loop(args, shutdown_rx));

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

        let stats = metrics_task.await??;

        info!(
            "metrics: exit: reconnects={}, send_errors={}, dropped_batches={}",
            stats.reconnects, stats.send_errors, stats.dropped_batches
        );

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn start_metrics_thread(
    args: cli::MetricsArgs,
    shutdown_rx: watch::Receiver<bool>,
) -> std::thread::JoinHandle<Result<SenderStats>> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        rt.block_on(run_metrics_loop(args, shutdown_rx))
    })
}

async fn run_metrics_loop(
    args: cli::MetricsArgs,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<SenderStats> {
    let (tx, rx) = mpsc::channel::<metrics::MetricsBatch>(args.queue_cap);

    let generator = tokio::spawn(run_generator(
        tx,
        shutdown_rx.clone(),
        args.source_id.clone(),
        Duration::from_millis(args.interval_ms),
        args.ops_per_tick,
    ));

    let auth_token = args.auth_token.trim().to_string();
    let auth_token = if auth_token.is_empty() {
        None
    } else {
        Some(auth_token)
    };

    let rpc_timeout = if args.rpc_timeout_ms == 0 {
        None
    } else {
        Some(Duration::from_millis(args.rpc_timeout_ms))
    };

    let sender_cfg = SenderConfig {
        socket_path: args.socket_path.clone(),
        connect_timeout: Duration::from_millis(args.connect_timeout_ms),
        rpc_timeout,
        backoff_initial: Duration::from_millis(args.backoff_initial_ms),
        backoff_max: Duration::from_millis(args.backoff_max_ms),
        jitter_ratio: args.jitter_ratio,
        conn_buffer: args.conn_buffer,
        shutdown_grace: Duration::from_millis(args.shutdown_grace_ms),
        auth_token,
    };

    let mut sender_task = tokio::spawn(run_sender(rx, shutdown_rx.clone(), sender_cfg));

    tokio::select! {
        res = &mut sender_task => {
            let _ = generator.await;
            let stats = res?;
            Ok(stats)
        }
        _ = wait_for_shutdown(shutdown_rx) => {
            let _ = generator.await;
            let stats = sender_task.await?;
            Ok(stats)
        }
    }
}

async fn wait_for_shutdown(mut shutdown: watch::Receiver<bool>) {
    loop {
        if *shutdown.borrow() {
            break;
        }
        if shutdown.changed().await.is_err() {
            break;
        }
    }
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
    use tokio::signal::unix::{SignalKind, signal};
    let mut s = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    s.recv().await;
}
