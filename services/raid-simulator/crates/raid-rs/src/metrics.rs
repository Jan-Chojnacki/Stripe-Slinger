use std::sync::{Arc, OnceLock};

#[derive(Copy, Clone, Debug)]
pub enum IoOpType {
    Read,
    Write,
}

#[derive(Clone, Debug)]
pub struct DiskOp {
    pub disk_id: String,
    pub op: IoOpType,
    pub bytes: u64,
    pub latency_seconds: f64,
    pub error: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct RaidOp {
    pub op: IoOpType,
    pub bytes: u64,
    pub latency_seconds: f64,
    pub error: bool,
}

pub trait MetricsSink: Send + Sync + 'static {
    fn record_disk_op(&self, op: DiskOp);
    fn record_raid_op(&self, op: RaidOp);
}

static METRICS_SINK: OnceLock<Arc<dyn MetricsSink>> = OnceLock::new();

pub fn install_metrics_sink(sink: Arc<dyn MetricsSink>) -> bool {
    METRICS_SINK.set(sink).is_ok()
}

pub fn is_enabled() -> bool {
    METRICS_SINK.get().is_some()
}

pub fn record_disk_op(op: DiskOp) {
    if let Some(sink) = METRICS_SINK.get() {
        sink.record_disk_op(op);
    }
}

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

        let disk_ops = sink.disk_ops.lock().unwrap();
        assert_eq!(disk_ops.len(), 1);
        assert_eq!(disk_ops[0].disk_id, "disk1");
        assert_eq!(disk_ops[0].bytes, 2048);
        assert!(!disk_ops[0].error);

        let raid_ops = sink.raid_ops.lock().unwrap();
        assert_eq!(raid_ops.len(), 1);
        assert_eq!(raid_ops[0].bytes, 512);
        assert!(raid_ops[0].error);
    }
}
