//! Logical volume management built on top of disk arrays and stripe layouts.

mod mapper;
#[cfg(test)]
mod mapper_tests;
#[cfg(test)]
mod volume_tests;

use anyhow::Result;
use mapper::{Geometry, geometry, locate_byte, stripe_byte_offset};

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::metrics::{IoOpType, RaidOp};
use crate::retention::array::Array;
use std::time::Instant;

/// `DiskStatus` summarizes the health of a disk within the volume.
#[derive(Copy, Clone, Debug)]
pub struct DiskStatus {
    pub index: usize,
    pub missing: bool,
    pub needs_rebuild: bool,
}

/// Volume combines a disk array with a stripe layout for logical IO.
pub struct Volume<const D: usize, const N: usize, T: Stripe<D, N>> {
    array: Array<D, N>,
    layout: T,
    geom: Geometry,
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> Volume<D, N, T> {
    /// `new` constructs a `Volume` from a disk array and stripe layout.
    ///
    /// # Arguments
    /// * `array` - Disk array backing the volume.
    /// * `layout` - Stripe layout implementation.
    pub fn new(array: Array<D, N>, layout: T) -> Self {
        Self {
            array,
            geom: geometry::<D, N, T>(),
            layout,
        }
    }

    /// `disk_status_string` returns a human-readable status summary.
    pub fn disk_status_string(&self) -> String {
        self.array.status_string()
    }

    /// `fail_disk` marks the disk at the given index as failed.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to fail.
    ///
    /// # Errors
    /// Returns an error if the disk cannot be failed.
    pub fn fail_disk(&mut self, i: usize) -> Result<()> {
        self.array.fail_disk(i)
    }

    /// `replace_disk` replaces the disk image at the given index.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to replace.
    ///
    /// # Errors
    /// Returns an error if the disk cannot be replaced.
    pub fn replace_disk(&mut self, i: usize) -> Result<()> {
        self.array.replace_disk(i)
    }

    /// `any_needs_rebuild` reports whether any disk needs rebuild work.
    pub fn any_needs_rebuild(&self) -> bool {
        self.array
            .0
            .iter()
            .any(|d| d.needs_rebuild && !d.is_missing())
    }

    /// `failed_disks` returns the number of missing disks.
    pub fn failed_disks(&self) -> u32 {
        self.array
            .0
            .iter()
            .filter(|d| d.is_missing())
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    /// `disk_statuses` returns a list of disk status summaries.
    pub fn disk_statuses(&self) -> Vec<DiskStatus> {
        self.array
            .0
            .iter()
            .enumerate()
            .map(|(index, disk)| DiskStatus {
                index,
                missing: disk.is_missing(),
                needs_rebuild: disk.needs_rebuild,
            })
            .collect()
    }

    /// `logical_capacity_bytes` returns the logical data capacity of the volume.
    pub fn logical_capacity_bytes(&self) -> u64 {
        self.array.disk_len().saturating_mul(T::DATA as u64)
    }

    /// `stripes_needed_for_logical_end` returns the stripe count for the given logical end.
    ///
    /// # Arguments
    /// * `logical_end` - Logical byte position at the end of interest.
    pub fn stripes_needed_for_logical_end(&self, logical_end: u64) -> u64 {
        let bytes_per_stripe = (T::DATA as u64).saturating_mul(N as u64);
        if bytes_per_stripe == 0 {
            return 0;
        }
        let end = logical_end.min(self.logical_capacity_bytes());
        if end == 0 {
            return 0;
        }
        end.div_ceil(bytes_per_stripe)
    }

    /// `repair_stripe` forces a stripe read to rebuild missing data.
    ///
    /// # Arguments
    /// * `stripe_index` - Index of the stripe to repair.
    pub fn repair_stripe(&mut self, stripe_index: u64) {
        self.load_stripe(stripe_index);
    }

    /// `clear_needs_rebuild_all` clears rebuild flags on all operational disks.
    pub fn clear_needs_rebuild_all(&mut self) {
        for d in &mut self.array.0 {
            if !d.is_missing() {
                d.needs_rebuild = false;
            }
        }
    }

    /// `clear_needs_rebuild_disk` clears the rebuild flag for a specific disk.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to clear.
    pub fn clear_needs_rebuild_disk(&mut self, i: usize) {
        if i < D && !self.array.0[i].is_missing() {
            self.array.0[i].needs_rebuild = false;
        }
    }

    /// `rebuild` triggers a best-effort rebuild across all disks.
    pub fn rebuild(&mut self) {
        let _ = self.rebuild_all();
    }

    /// `rebuild_all_upto` rebuilds stripes up to the provided logical end.
    ///
    /// # Arguments
    /// * `logical_end` - Logical byte position to rebuild up to.
    ///
    /// # Errors
    /// Returns an error if rebuilding fails.
    pub fn rebuild_all_upto(&mut self, logical_end: u64) -> Result<()> {
        if self.layout.as_restore().is_none() {
            return Ok(());
        }
        if !self.any_needs_rebuild() {
            return Ok(());
        }

        let stripes = self.stripes_needed_for_logical_end(logical_end);
        for s in 0..stripes {
            self.load_stripe(s);
        }

        self.clear_needs_rebuild_all();
        Ok(())
    }

    /// `rebuild_disk_upto` rebuilds a specific disk up to the provided logical end.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to rebuild.
    /// * `logical_end` - Logical byte position to rebuild up to.
    ///
    /// # Errors
    /// Returns an error if rebuilding fails.
    pub fn rebuild_disk_upto(&mut self, i: usize, logical_end: u64) -> Result<()> {
        if i >= D {
            anyhow::bail!("disk index out of range: {i} (D={D})");
        }
        if self.layout.as_restore().is_none() {
            return Ok(());
        }
        if self.array.0[i].is_missing() {
            anyhow::bail!("disk {i} is missing/failed; replace it first");
        }
        if !self.array.0[i].needs_rebuild {
            return Ok(());
        }

        let stripes = self.stripes_needed_for_logical_end(logical_end);
        for s in 0..stripes {
            self.load_stripe(s);
        }

        self.clear_needs_rebuild_disk(i);
        Ok(())
    }

    /// `rebuild_all` rebuilds all disks across the full logical range.
    ///
    /// # Errors
    /// Returns an error if rebuilding fails.
    pub fn rebuild_all(&mut self) -> Result<()> {
        self.rebuild_all_upto(self.logical_capacity_bytes())
    }

    /// `rebuild_disk` rebuilds a single disk across the full logical range.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to rebuild.
    ///
    /// # Errors
    /// Returns an error if rebuilding fails.
    pub fn rebuild_disk(&mut self, i: usize) -> Result<()> {
        self.rebuild_disk_upto(i, self.logical_capacity_bytes())
    }

    /// `write_bytes` writes payload bytes into the volume at the logical offset.
    ///
    /// # Arguments
    /// * `byte_offset` - Logical byte offset within the volume.
    /// * `payload` - Bytes to write.
    pub fn write_bytes(&mut self, byte_offset: u64, payload: &[u8]) {
        let start = crate::metrics::is_enabled().then(Instant::now);
        let mut data_chunks = vec![Bits::<N>::zero(); T::DATA];

        let mut written: usize = 0;
        let total = payload.len();
        while written < total {
            let (stripe_index, in_stripe_byte) = locate_byte(byte_offset, written, &self.geom);
            let stripe_bytes = self.geom.bytes_per_stripe - in_stripe_byte;
            let take = stripe_bytes.min(total - written);

            self.load_stripe(stripe_index);

            self.layout.read(&mut data_chunks);

            for i in 0..take {
                let byte_in_stripe = in_stripe_byte + i;
                let chunk_index = byte_in_stripe / self.geom.bytes_per_chunk;
                let byte_index = byte_in_stripe % self.geom.bytes_per_chunk;
                data_chunks[chunk_index].as_bytes_mut()[byte_index] = payload[written + i];
            }

            self.layout.write(&data_chunks);
            self.store_stripe(stripe_index);
            written += take;
        }

        if let Some(start) = start {
            let bytes = u64::try_from(payload.len()).unwrap_or(u64::MAX);
            crate::metrics::record_raid_op(RaidOp {
                op: IoOpType::Write,
                bytes,
                latency_seconds: start.elapsed().as_secs_f64(),
                error: false,
            });
        }
    }

    /// `read_bytes` reads bytes from the volume into the output buffer.
    ///
    /// # Arguments
    /// * `byte_offset` - Logical byte offset within the volume.
    /// * `out` - Output buffer to populate.
    pub fn read_bytes(&mut self, byte_offset: u64, out: &mut [u8]) {
        let start = crate::metrics::is_enabled().then(Instant::now);
        let mut data_chunks = vec![Bits::<N>::zero(); T::DATA];

        let mut read: usize = 0;
        let total = out.len();
        while read < total {
            let (stripe_index, in_stripe_byte) = locate_byte(byte_offset, read, &self.geom);
            let stripe_bytes = self.geom.bytes_per_stripe - in_stripe_byte;
            let take = stripe_bytes.min(total - read);

            self.load_stripe(stripe_index);

            self.layout.read(&mut data_chunks);

            for i in 0..take {
                let byte_in_stripe = in_stripe_byte + i;
                let chunk_index = byte_in_stripe / self.geom.bytes_per_chunk;
                let byte_index = byte_in_stripe % self.geom.bytes_per_chunk;
                out[read + i] = data_chunks[chunk_index].as_bytes()[byte_index];
            }

            read += take;
        }

        if let Some(start) = start {
            let bytes = u64::try_from(out.len()).unwrap_or(u64::MAX);
            crate::metrics::record_raid_op(RaidOp {
                op: IoOpType::Read,
                bytes,
                latency_seconds: start.elapsed().as_secs_f64(),
                error: false,
            });
        }
    }

    fn load_stripe(&mut self, stripe_index: u64) {
        let byte_offset = stripe_byte_offset::<N>(stripe_index);
        self.array.read(byte_offset, &mut self.layout);
    }

    fn store_stripe(&mut self, stripe_index: u64) {
        let byte_offset = stripe_byte_offset::<N>(stripe_index);
        self.array.write(byte_offset, &self.layout);
    }
}
