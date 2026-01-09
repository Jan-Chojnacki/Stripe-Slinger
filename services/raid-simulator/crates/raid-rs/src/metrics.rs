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
