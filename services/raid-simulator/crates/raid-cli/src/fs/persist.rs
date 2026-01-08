use raid_rs::layout::stripe::traits::stripe::Stripe;

use super::constants::{ENTRY_SIZE, HEADER_SIZE};
use super::raidfs::{FsState, RaidFs};

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
