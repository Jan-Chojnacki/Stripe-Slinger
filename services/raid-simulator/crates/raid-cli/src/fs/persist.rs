//! Persistence helpers for RAID filesystem metadata.

use raid_rs::layout::stripe::traits::stripe::Stripe;

use super::constants::{ENTRY_SIZE, HEADER_SIZE};
use super::raidfs::{FsState, RaidFs};

/// save_header_and_entry writes the header and a single entry back to disk.
///
/// # Arguments
/// * `state` - Filesystem state to persist.
/// * `index` - Entry index to save.
pub fn save_header_and_entry<const D: usize, const N: usize, T: Stripe<D, N>>(
    state: &mut FsState<D, N, T>,
    index: usize,
) {
    let header_bytes = RaidFs::<D, N, T>::header_bytes(&state.header);
    state.volume.write_bytes(0, &header_bytes);
    let entry_bytes = state.entries[index].to_bytes();
    let entry_offset = HEADER_SIZE as u64 + (index as u64 * ENTRY_SIZE as u64);
    state.volume.write_bytes(entry_offset, &entry_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::constants::{ENTRY_SIZE, HEADER_SIZE};
    use crate::fs::metadata::Entry;
    use crate::fs::test_utils::{TestStripe, create_test_state};

    #[test]
    fn save_header_and_entry_persists_bytes() {
        let mut state = create_test_state();
        state.header.next_free = 1234;
        state.entries[0] = Entry {
            name: "file.txt".to_string(),
            offset: 200,
            size: 12,
            used: true,
        };

        save_header_and_entry(&mut state, 0);

        let mut header_buf = [0u8; HEADER_SIZE];
        state.volume.read_bytes(0, &mut header_buf);
        let parsed = RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::parse_header(&header_buf)
            .expect("header parsed");
        assert_eq!(parsed.next_free, 1234);

        let mut entry_buf = [0u8; ENTRY_SIZE];
        let entry_offset = HEADER_SIZE as u64;
        state.volume.read_bytes(entry_offset, &mut entry_buf);
        let parsed_entry = Entry::from_bytes(&entry_buf);
        assert_eq!(parsed_entry.name, "file.txt");
        assert_eq!(parsed_entry.offset, 200);
        assert_eq!(parsed_entry.size, 12);
        assert!(parsed_entry.used);
    }
}
