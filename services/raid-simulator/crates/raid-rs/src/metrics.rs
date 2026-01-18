//! Lightweight metrics hooks for recording RAID simulator events.

use std::sync::{Arc, OnceLock};

/// `IoOpType` describes a read or write operation.
#[derive(Copy, Clone, Debug)]
pub enum IoOpType {
    Read,
    Write,
}

/// `DiskOp` captures disk IO metrics emitted by the simulator.
#[derive(Clone, Debug)]
pub struct DiskOp {
    pub disk_id: String,
    pub op: IoOpType,
    pub bytes: u64,
    pub latency_seconds: f64,
    pub error: bool,
}

/// `RaidOp` captures RAID IO metrics emitted by the simulator.
#[derive(Copy, Clone, Debug)]
pub struct RaidOp {
    pub op: IoOpType,
    pub bytes: u64,
    pub latency_seconds: f64,
    pub error: bool,
}

/// `MetricsSink` records disk and RAID operations from the simulator.
pub trait MetricsSink: Send + Sync + 'static {
    /// `record_disk_op` records a disk IO event.
    fn record_disk_op(&self, op: DiskOp);
    /// `record_raid_op` records a RAID IO event.
    fn record_raid_op(&self, op: RaidOp);
}

static METRICS_SINK: OnceLock<Arc<dyn MetricsSink>> = OnceLock::new();

/// `install_metrics_sink` installs a global metrics sink for the simulator.
///
/// # Arguments
/// * `sink` - Sink implementation to register.
///
/// # Returns
/// `true` if the sink was installed, `false` if one was already registered.
pub fn install_metrics_sink(sink: Arc<dyn MetricsSink>) -> bool {
    METRICS_SINK.set(sink).is_ok()
}

/// `is_enabled` reports whether a metrics sink has been installed.
pub fn is_enabled() -> bool {
    METRICS_SINK.get().is_some()
}

/// `record_disk_op` forwards a disk operation to the installed sink.
///
/// # Arguments
/// * `op` - Disk operation to record.
pub fn record_disk_op(op: DiskOp) {
    if let Some(sink) = METRICS_SINK.get() {
        sink.record_disk_op(op);
    }
}

/// `record_raid_op` forwards a RAID operation to the installed sink.
///
/// # Arguments
/// * `op` - RAID operation to record.
pub fn record_raid_op(op: RaidOp) {
    if let Some(sink) = METRICS_SINK.get() {
        sink.record_raid_op(op);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct TestSink {
        disk_ops: Mutex<Vec<DiskOp>>,
        raid_ops: Mutex<Vec<RaidOp>>,
    }

    impl MetricsSink for TestSink {
        fn record_disk_op(&self, op: DiskOp) {
            self.disk_ops.lock().unwrap().push(op);
        }

        fn record_raid_op(&self, op: RaidOp) {
            self.raid_ops.lock().unwrap().push(op);
        }
    }

    #[test]
    fn metrics_sink_records_ops_when_enabled() {
        let sink = Arc::new(TestSink {
            disk_ops: Mutex::new(Vec::new()),
            raid_ops: Mutex::new(Vec::new()),
        });

        assert!(install_metrics_sink(sink.clone()));
        assert!(is_enabled());

        record_disk_op(DiskOp {
            disk_id: "disk1".to_string(),
            op: IoOpType::Write,
            bytes: 2048,
            latency_seconds: 0.15,
            error: false,
        });
        record_raid_op(RaidOp {
            op: IoOpType::Read,
            bytes: 512,
            latency_seconds: 0.05,
            error: true,
        });

        {
            let disk_ops = sink.disk_ops.lock().unwrap();
            assert_eq!(disk_ops.len(), 1);
            assert_eq!(disk_ops[0].disk_id, "disk1");
            assert_eq!(disk_ops[0].bytes, 2048);
            assert!(!disk_ops[0].error);
            drop(disk_ops);
        }

        {
            let raid_ops = sink.raid_ops.lock().unwrap();
            assert_eq!(raid_ops.len(), 1);
            assert_eq!(raid_ops[0].bytes, 512);
            assert!(raid_ops[0].error);
            drop(raid_ops);
        }
    }
}
