pub mod constants;
pub mod metadata;
pub mod persist;
pub mod raidfs;

pub use constants::*;
pub use metadata::{Entry, Header};
pub use raidfs::{FsState, RaidFs};

#[cfg(test)]
pub(crate) mod test_utils {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use raid_rs::layout::stripe::raid0::RAID0;
    use raid_rs::retention::array::Array;
    use raid_rs::retention::volume::Volume;

    use super::constants::{DEFAULT_CHUNK_SIZE, MAX_FILES};
    use super::metadata::{Entry, Header};
    use super::raidfs::{FsState, RaidFs};

    pub type TestStripe = RAID0<1, { DEFAULT_CHUNK_SIZE }>;
    pub type TestState = FsState<1, { DEFAULT_CHUNK_SIZE }, TestStripe>;
    pub type TestFs = RaidFs<1, { DEFAULT_CHUNK_SIZE }, TestStripe>;

    pub fn temp_dir(prefix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        dir.push(format!("{prefix}-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    pub fn create_test_state() -> TestState {
        let dir = temp_dir("raid-cli");
        let paths = [dir.join("disk-0.img").to_string_lossy().into_owned()];
        let array = Array::<1, { DEFAULT_CHUNK_SIZE }>::init_array(&paths, 20_000);
        let volume = Volume::new(array, TestStripe::zero());
        let header = Header {
            next_free: RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::data_start(),
        };
        let entries = vec![Entry::empty(); MAX_FILES];
        TestState {
            volume,
            header,
            entries,
        }
    }

    pub fn create_test_fs() -> TestFs {
        let state = create_test_state();
        let capacity = state.volume.logical_capacity_bytes();
        TestFs {
            state: Arc::new(Mutex::new(state)),
            capacity,
            metrics: None,
        }
    }
}
