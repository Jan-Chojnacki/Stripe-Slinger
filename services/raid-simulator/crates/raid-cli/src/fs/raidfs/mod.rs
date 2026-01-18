//! RAID-backed filesystem implementation for the FUSE layer.

mod core;
mod filesystem;
mod ops_attr;
mod ops_create;
mod ops_dir;
mod ops_io;
mod ops_sync;
mod types;

pub use types::{FsState, RaidFs};

#[cfg(test)]
mod tests {
    use super::RaidFs;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::test_utils::TestStripe;

    #[test]
    fn raidfs_data_start_is_nonzero() {
        let start = RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::data_start();
        assert!(start > 0);
    }
}
