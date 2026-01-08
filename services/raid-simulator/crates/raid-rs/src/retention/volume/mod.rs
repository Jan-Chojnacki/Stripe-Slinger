mod mapper;
#[cfg(test)]
mod mapper_tests;
#[cfg(test)]
mod volume_tests;

use anyhow::Result;
use mapper::{Geometry, geometry, locate_byte, stripe_byte_offset};

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::retention::array::Array;

pub struct Volume<const D: usize, const N: usize, T: Stripe<D, N>> {
    array: Array<D, N>,
    layout: T,
    geom: Geometry,
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> Volume<D, N, T> {
    pub fn new(array: Array<D, N>, layout: T) -> Self {
        Self {
            array,
            geom: geometry::<D, N, T>(),
            layout,
        }
    }

    pub fn disk_status_string(&self) -> String {
        self.array.status_string()
    }

    pub fn fail_disk(&mut self, i: usize) -> Result<()> {
        self.array.fail_disk(i)
    }

    pub fn replace_disk(&mut self, i: usize) -> Result<()> {
        self.array.replace_disk(i)
    }

    pub fn any_needs_rebuild(&self) -> bool {
        self.array
            .0
            .iter()
            .any(|d| d.needs_rebuild && !d.is_missing())
    }

    pub fn logical_capacity_bytes(&self) -> u64 {
        self.array.disk_len().saturating_mul(T::DATA as u64)
    }

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

    pub fn repair_stripe(&mut self, stripe_index: u64) {
        self.load_stripe(stripe_index);
    }

    pub fn clear_needs_rebuild_all(&mut self) {
        for d in &mut self.array.0 {
            if !d.is_missing() {
                d.needs_rebuild = false;
            }
        }
    }

    pub fn clear_needs_rebuild_disk(&mut self, i: usize) {
        if i < D && !self.array.0[i].is_missing() {
            self.array.0[i].needs_rebuild = false;
        }
    }

    pub fn rebuild(&mut self) {
        let _ = self.rebuild_all();
    }

    /// This scans only the requested logical prefix (typically: filesystem metadata + used data),
    /// relying on `Array::read()` read-repair writeback to populate missing/untrusted chunks.
    ///
    /// # Errors
    /// Returns an error if any disk rebuild operation fails.
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

    /// Rebuild a single disk (must exist and be operational), up to a logical byte offset.
    ///
    /// # Errors
    /// Returns an error if the disk index is invalid, missing, or rebuild fails.
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

    /// Rebuild all disks that are present but marked as `needs_rebuild`.
    ///
    /// This is a full scan (logical capacity). Prefer [`Self::rebuild_all_upto`] from call sites
    /// that know the "used" end of data (e.g. filesystem `next_free`) to avoid long startup times.
    ///
    /// # Errors
    /// Returns an error if any disk rebuild operation fails.
    pub fn rebuild_all(&mut self) -> Result<()> {
        self.rebuild_all_upto(self.logical_capacity_bytes())
    }

    /// Rebuild a single disk (must exist and be operational).
    ///
    /// This is a full scan (logical capacity). Prefer [`Self::rebuild_disk_upto`] when possible.
    ///
    /// # Errors
    /// Returns an error if the disk index is invalid, missing, or rebuild fails.
    pub fn rebuild_disk(&mut self, i: usize) -> Result<()> {
        self.rebuild_disk_upto(i, self.logical_capacity_bytes())
    }

    pub fn write_bytes(&mut self, byte_offset: u64, payload: &[u8]) {
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
    }

    pub fn read_bytes(&mut self, byte_offset: u64, out: &mut [u8]) {
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
