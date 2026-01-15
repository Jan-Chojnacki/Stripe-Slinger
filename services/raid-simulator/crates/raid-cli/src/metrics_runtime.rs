use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use prost_types::Timestamp;
use tokio::sync::{mpsc, watch};
use tracing::warn;

use raid_rs::metrics::{DiskOp, IoOpType, MetricsSink, RaidOp};
use raid_rs::retention::volume::DiskStatus;

use crate::cli::MetricsArgs;
use crate::pb::metrics;
use crate::sender::{SenderConfig, SenderStats, run_sender};

#[derive(Copy, Clone, Debug)]
pub enum FuseOpType {
    Read,
    Write,
    Open,
    Fsync,
}

#[derive(Clone, Debug)]
pub struct FuseOp {
    pub op: FuseOpType,
    pub bytes: u64,
    pub latency_seconds: f64,
    pub error: bool,
}

#[derive(Clone, Debug)]
pub enum MetricsEvent {
    DiskOp(DiskOp),
    RaidOp { raid_id: String, op: RaidOp },
    FuseOp(FuseOp),
    DiskState(metrics::DiskState),
    RaidState(metrics::RaidState),
}

#[derive(Clone)]
pub struct MetricsEmitter {
    raid_id: String,
    tx: mpsc::Sender<MetricsEvent>,
}

impl MetricsEmitter {
    pub fn new(raid_id: String, tx: mpsc::Sender<MetricsEvent>) -> Arc<Self> {
        Arc::new(Self { raid_id, tx })
    }

    pub fn record_fuse_op(&self, op: FuseOp) {
        let _ = self.tx.try_send(MetricsEvent::FuseOp(op));
    }

    pub fn record_disk_status(&self, status: DiskStatus) {
        let disk_id = format!("disk{}", status.index);
        let queue_depth = if status.missing {
            -1.0
        } else if status.needs_rebuild {
            1.0
        } else {
            0.0
        };
        let _ = self
            .tx
            .try_send(MetricsEvent::DiskState(metrics::DiskState {
                disk_id,
                queue_depth,
            }));
    }

    pub fn record_raid_state(&self, failed_disks: u32, rebuild_in_progress: bool, progress: f64) {
        let state = metrics::RaidState {
            raid_id: self.raid_id.clone(),
            raid1_resync_progress: progress,
            degraded: failed_disks > 0,
            failed_disks,
            rebuild_in_progress,
        };
        let _ = self.tx.try_send(MetricsEvent::RaidState(state));
    }
}

impl MetricsSink for MetricsEmitter {
    fn record_disk_op(&self, op: DiskOp) {
        let _ = self.tx.try_send(MetricsEvent::DiskOp(op));
    }

    fn record_raid_op(&self, op: RaidOp) {
        let _ = self.tx.try_send(MetricsEvent::RaidOp {
            raid_id: self.raid_id.clone(),
            op,
        });
    }
}

pub async fn run_event_metrics_loop(
    args: MetricsArgs,
    mut shutdown_rx: watch::Receiver<bool>,
    event_rx: mpsc::Receiver<MetricsEvent>,
) -> Result<SenderStats> {
    let (tx, rx) = mpsc::channel::<metrics::MetricsBatch>(args.queue_cap);

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
    let mut generator_task = tokio::spawn(run_event_generator(
        tx,
        shutdown_rx.clone(),
        event_rx,
        args.source_id.clone(),
        Duration::from_millis(args.interval_ms),
    ));

    tokio::select! {
        res = &mut sender_task => {
            let _ = generator_task.await;
            let stats = res?;
            Ok(stats)
        }
        () = wait_for_shutdown(shutdown_rx) => {
            let _ = generator_task.await;
            let stats = sender_task.await?;
            Ok(stats)
        }
    }
}

async fn run_event_generator(
    tx: mpsc::Sender<metrics::MetricsBatch>,
    mut shutdown: watch::Receiver<bool>,
    mut event_rx: mpsc::Receiver<MetricsEvent>,
    source_id: String,
    interval: Duration,
) {
    let mut seq_no: u64 = 1;
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut dropped: u64 = 0;
    let mut disk_state_cache: HashMap<String, metrics::DiskState> = HashMap::new();
    let mut raid_state_cache: HashMap<String, metrics::RaidState> = HashMap::new();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let mut disk_ops = Vec::new();
                let mut raid_ops = Vec::new();
                let mut fuse_ops = Vec::new();
                let mut disk_ids = HashSet::new();

                while let Ok(event) = event_rx.try_recv() {
                    match event {
                        MetricsEvent::DiskOp(op) => {
                            disk_ids.insert(op.disk_id.clone());
                            disk_ops.push(to_disk_op(op));
                        }
                        MetricsEvent::RaidOp { raid_id, op } => {
                            raid_ops.push(to_raid_op(&raid_id, op));
                        }
                        MetricsEvent::FuseOp(op) => {
                            fuse_ops.push(to_fuse_op(op));
                        }
                        MetricsEvent::DiskState(state) => {
                            disk_state_cache.insert(state.disk_id.clone(), state);
                        }
                        MetricsEvent::RaidState(state) => {
                            raid_state_cache.insert(state.raid_id.clone(), state);
                        }
                    }
                }

                let mut disk_states = disk_state_cache.values().cloned().collect::<Vec<_>>();
                for disk_id in disk_ids {
                    if !disk_state_cache.contains_key(&disk_id) {
                        disk_states.push(metrics::DiskState {
                            disk_id,
                            queue_depth: 0.0,
                        });
                    }
                }

                let raid_states = raid_state_cache.values().cloned().collect::<Vec<_>>();

                let process = process_sample();

                if disk_ops.is_empty()
                    && raid_ops.is_empty()
                    && fuse_ops.is_empty()
                    && disk_states.is_empty()
                    && raid_states.is_empty()
                    && process.is_none()
                {
                    continue;
                }

                let batch = metrics::MetricsBatch {
                    source_id: source_id.clone(),
                    seq_no,
                    timestamp: Some(now_ts()),
                    disk_ops,
                    disk_states,
                    raid_ops,
                    raid_states,
                    fuse_ops,
                    process,
                };
                seq_no = seq_no.wrapping_add(1);

                match tx.try_send(batch) {
                    Ok(()) => {}
                    Err(_e) => {
                        dropped += 1;
                        if dropped.is_multiple_of(100) {
                            warn!("generator: dropped_batches={}", dropped);
                        }
                    }
                }
            },
            changed = shutdown.changed() => {
                if changed.is_err() {
                    break;
                }
                if *shutdown.borrow() {
                    break;
                }
            },
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

fn to_disk_op(op: DiskOp) -> metrics::DiskOp {
    metrics::DiskOp {
        disk_id: op.disk_id,
        op: to_io_op(op.op),
        bytes: op.bytes,
        latency_seconds: op.latency_seconds,
        error: op.error,
    }
}

fn to_raid_op(raid_id: &str, op: RaidOp) -> metrics::RaidOp {
    metrics::RaidOp {
        raid_id: raid_id.to_string(),
        op: to_io_op(op.op),
        bytes: op.bytes,
        latency_seconds: op.latency_seconds,
        error: op.error,
        served_from_disk_id: String::new(),
        raid3_parity_read: false,
        raid3_parity_write: false,
        raid3_partial_stripe_write: false,
    }
}

fn to_fuse_op(op: FuseOp) -> metrics::FuseOp {
    let op_type = match op.op {
        FuseOpType::Read => metrics::FuseOpType::FuseOpRead,
        FuseOpType::Write => metrics::FuseOpType::FuseOpWrite,
        FuseOpType::Open => metrics::FuseOpType::FuseOpOpen,
        FuseOpType::Fsync => metrics::FuseOpType::FuseOpFsync,
    };
    metrics::FuseOp {
        op: op_type as i32,
        bytes: op.bytes,
        latency_seconds: op.latency_seconds,
        error: op.error,
    }
}

fn to_io_op(op: IoOpType) -> i32 {
    match op {
        IoOpType::Read => metrics::IoOpType::IoOpRead as i32,
        IoOpType::Write => metrics::IoOpType::IoOpWrite as i32,
    }
}

fn now_ts() -> Timestamp {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp {
        seconds: i64::try_from(dur.as_secs()).unwrap_or(i64::MAX),
        nanos: i32::try_from(dur.subsec_nanos()).unwrap_or(i32::MAX),
    }
}

#[cfg(unix)]
fn process_sample() -> Option<metrics::ProcessSample> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    let user = timeval_to_secs(usage.ru_utime);
    let sys = timeval_to_secs(usage.ru_stime);
    let cpu_seconds = user + sys;
    let resident_memory_bytes = u64::try_from(usage.ru_maxrss)
        .unwrap_or(0)
        .saturating_mul(1024);

    Some(metrics::ProcessSample {
        cpu_seconds,
        resident_memory_bytes,
    })
}

#[cfg(not(unix))]
fn process_sample() -> Option<metrics::ProcessSample> {
    None
}

#[cfg(unix)]
fn timeval_to_secs(tv: libc::timeval) -> f64 {
    let secs = tv.tv_sec as f64;
    let micros = tv.tv_usec as f64 / 1_000_000.0;
    secs + micros
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::timeout;

    #[tokio::test]
    async fn metrics_emitter_records_disk_status_queue_depths() {
        let (tx, mut rx) = mpsc::channel(10);
        let emitter = MetricsEmitter::new("raid1".to_string(), tx);

        emitter.record_disk_status(DiskStatus {
            index: 0,
            missing: true,
            needs_rebuild: false,
        });
        emitter.record_disk_status(DiskStatus {
            index: 1,
            missing: false,
            needs_rebuild: true,
        });
        emitter.record_disk_status(DiskStatus {
            index: 2,
            missing: false,
            needs_rebuild: false,
        });

        let mut states = Vec::new();
        for _ in 0..3 {
            match rx.recv().await {
                Some(MetricsEvent::DiskState(state)) => states.push(state),
                other => panic!("expected DiskState event, got {other:?}"),
            }
        }

        let mut queue_depths = HashMap::new();
        for state in states {
            queue_depths.insert(state.disk_id, state.queue_depth);
        }

        assert_eq!(queue_depths.get("disk0"), Some(&-1.0));
        assert_eq!(queue_depths.get("disk1"), Some(&1.0));
        assert_eq!(queue_depths.get("disk2"), Some(&0.0));
    }

    #[tokio::test]
    async fn run_event_generator_batches_ops_and_states() {
        let (batch_tx, mut batch_rx) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (event_tx, event_rx) = mpsc::channel(10);

        let task = tokio::spawn(run_event_generator(
            batch_tx,
            shutdown_rx,
            event_rx,
            "source-1".to_string(),
            Duration::from_millis(5),
        ));

        event_tx
            .send(MetricsEvent::DiskOp(DiskOp {
                disk_id: "disk0".to_string(),
                op: IoOpType::Read,
                bytes: 64,
                latency_seconds: 0.02,
                error: false,
            }))
            .await
            .unwrap();
        event_tx
            .send(MetricsEvent::DiskState(metrics::DiskState {
                disk_id: "disk1".to_string(),
                queue_depth: 1.0,
            }))
            .await
            .unwrap();
        event_tx
            .send(MetricsEvent::RaidOp {
                raid_id: "raid1".to_string(),
                op: RaidOp {
                    op: IoOpType::Read,
                    bytes: 12,
                    latency_seconds: 0.25,
                    error: false,
                },
            })
            .await
            .unwrap();
        event_tx
            .send(MetricsEvent::FuseOp(FuseOp {
                op: FuseOpType::Write,
                bytes: 128,
                latency_seconds: 0.04,
                error: true,
            }))
            .await
            .unwrap();

        let batch = timeout(Duration::from_millis(200), batch_rx.recv())
            .await
            .expect("batch send timeout")
            .expect("batch missing");

        assert_eq!(batch.source_id, "source-1");
        assert_eq!(batch.seq_no, 1);
        assert_eq!(batch.disk_ops.len(), 1);
        assert_eq!(batch.raid_ops.len(), 1);
        assert_eq!(batch.fuse_ops.len(), 1);

        let mut disk_states = HashMap::new();
        for state in batch.disk_states {
            disk_states.insert(state.disk_id, state.queue_depth);
        }
        assert_eq!(disk_states.get("disk0"), Some(&0.0));
        assert_eq!(disk_states.get("disk1"), Some(&1.0));

        let raid_op = &batch.raid_ops[0];
        assert_eq!(raid_op.raid_id, "raid1");
        assert_eq!(raid_op.op, metrics::IoOpType::IoOpRead as i32);
        assert_eq!(raid_op.bytes, 12);
        assert_eq!(raid_op.latency_seconds, 0.25);
        assert!(!raid_op.error);
        assert_eq!(raid_op.served_from_disk_id, "");
        assert!(!raid_op.raid3_parity_read);
        assert!(!raid_op.raid3_parity_write);
        assert!(!raid_op.raid3_partial_stripe_write);

        let fuse_op = &batch.fuse_ops[0];
        assert_eq!(fuse_op.op, metrics::FuseOpType::FuseOpWrite as i32);
        assert_eq!(fuse_op.bytes, 128);
        assert_eq!(fuse_op.latency_seconds, 0.04);
        assert!(fuse_op.error);

        let _ = shutdown_tx.send(true);
        let _ = timeout(Duration::from_millis(200), task).await;
    }
}
