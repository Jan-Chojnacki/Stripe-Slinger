//! Core filesystem state types for the RAID-backed FUSE layer.

use std::sync::{Arc, Mutex};

use raid_rs::layout::stripe::traits::stripe::Stripe;
use raid_rs::retention::volume::Volume;

use crate::fs::metadata::{Entry, Header};
use crate::metrics_runtime::MetricsEmitter;

/// FsState holds the mutable on-disk state for the filesystem.
pub struct FsState<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub volume: Volume<D, N, T>,
    pub header: Header,
    pub entries: Vec<Entry>,
}

/// RaidFs wraps shared state and capacity metadata for FUSE operations.
pub struct RaidFs<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub state: Arc<Mutex<FsState<D, N, T>>>,
    pub capacity: u64,
    pub metrics: Option<Arc<MetricsEmitter>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::test_utils::{TestStripe, create_test_fs, create_test_state};

    #[test]
    fn fs_state_can_be_built() {
        let state = create_test_state();
        assert!(state.entries.iter().all(|entry| !entry.used));
    }

    #[test]
    fn raidfs_has_expected_capacity() {
        let fs = create_test_fs();
        let capacity = fs.capacity;
        let state = fs.state.lock().expect("state lock");
        assert_eq!(capacity, state.volume.logical_capacity_bytes());
    }

    #[test]
    fn raidfs_can_store_metrics_handle() {
        let state = create_test_state();
        let fs = RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe> {
            state: Arc::new(Mutex::new(state)),
            capacity: 1,
            metrics: None,
        };
        assert!(fs.metrics.is_none());
    }
}
