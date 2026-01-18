//! Disk array helpers for reading and writing RAID stripes.

#[cfg(test)]
mod array_tests;

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::metrics::{DiskOp, IoOpType};
use crate::retention::disk::Disk;
use std::fmt::Write;
use std::time::Instant;

/// Array manages a fixed set of disk images for a RAID volume.
pub struct Array<const D: usize, const N: usize>(pub [Disk; D]);

impl<const D: usize, const N: usize> Array<D, N> {
    #[must_use]
    /// `init_array` creates and opens a disk array using the provided paths.
    ///
    /// # Arguments
    /// * `paths` - Disk image paths, one per disk.
    /// * `len` - Length of each disk image in bytes.
    ///
    /// # Panics
    /// Panics if any disk image cannot be created or opened.
    pub fn init_array(paths: &[String; D], len: u64) -> Self {
        let array: [Disk; D] =
            std::array::from_fn(|i| Disk::open_prealloc(&paths[i], len).unwrap());

        Self(array)
    }

    #[must_use]
    /// `disk_len` returns the length of the first disk in the array.
    pub fn disk_len(&self) -> u64 {
        self.0.first().map_or(0, Disk::len)
    }

    /// `fail_disk` simulates a disk failure at the specified index.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to fail.
    ///
    /// # Errors
    /// Returns an error if the index is out of range or the disk cannot fail.
    pub fn fail_disk(&mut self, i: usize) -> anyhow::Result<()> {
        if i >= D {
            anyhow::bail!("disk index out of range: {i} (D={D})");
        }
        self.0[i].fail()
    }

    /// `replace_disk` replaces the disk image at the specified index.
    ///
    /// # Arguments
    /// * `i` - Index of the disk to replace.
    ///
    /// # Errors
    /// Returns an error if the index is out of range or the disk cannot be replaced.
    pub fn replace_disk(&mut self, i: usize) -> anyhow::Result<()> {
        if i >= D {
            anyhow::bail!("disk index out of range: {i} (D={D})");
        }
        self.0[i].replace()
    }

    #[must_use]
    /// `status_string` returns a human-readable status summary for each disk.
    pub fn status_string(&self) -> String {
        let mut out = String::new();
        for (i, d) in self.0.iter().enumerate() {
            let state = if d.is_missing() {
                "FAILED"
            } else if d.needs_rebuild {
                "NEEDS_REBUILD"
            } else {
                "OK"
            };
            let exists = d.path().exists();
            let _ = writeln!(
                out,
                "disk {i}: {state} (image_exists={exists}, path={})",
                d.path().display()
            );
        }
        out
    }

    /// `write` persists a stripe to disk at the specified offset.
    ///
    /// # Arguments
    /// * `off` - Byte offset within each disk.
    /// * `stripe` - Stripe data to write.
    pub fn write<T: Stripe<D, N>>(&mut self, off: u64, stripe: &T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];
        stripe.read_raw(&mut data_buf);

        for (i, (disk, data)) in self.0.iter_mut().zip(&data_buf).enumerate() {
            if !disk.is_missing() {
                let start = crate::metrics::is_enabled().then(Instant::now);
                let written = disk.write_at(off, &data.0);
                if written == data.0.len() {
                    disk.needs_rebuild = false;
                }
                if let Some(start) = start {
                    let bytes = u64::try_from(data.0.len()).unwrap_or(u64::MAX);
                    let error = written != data.0.len();
                    crate::metrics::record_disk_op(DiskOp {
                        disk_id: format!("disk{i}"),
                        op: IoOpType::Write,
                        bytes,
                        latency_seconds: start.elapsed().as_secs_f64(),
                        error,
                    });
                }
            }
        }
    }

    /// `read` loads a stripe from disk at the specified offset.
    ///
    /// # Arguments
    /// * `off` - Byte offset within each disk.
    /// * `stripe` - Stripe object to populate.
    pub fn read<T: Stripe<D, N>>(&mut self, off: u64, stripe: &mut T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];

        let mut missing_or_untrusted: Vec<usize> = Vec::new();
        let supports_restore = stripe.as_restore().is_some();

        for (i, (disk, data)) in self.0.iter_mut().zip(data_buf.iter_mut()).enumerate() {
            let disk_missing = disk.is_missing();
            let untrusted = disk.needs_rebuild;

            if disk_missing || (supports_restore && untrusted) {
                missing_or_untrusted.push(i);
                continue;
            }
            let start = crate::metrics::is_enabled().then(Instant::now);
            let read = disk.read_at(off, &mut data.0);
            if let Some(start) = start {
                let bytes = u64::try_from(data.0.len()).unwrap_or(u64::MAX);
                let error = read != data.0.len();
                crate::metrics::record_disk_op(DiskOp {
                    disk_id: format!("disk{i}"),
                    op: IoOpType::Read,
                    bytes,
                    latency_seconds: start.elapsed().as_secs_f64(),
                    error,
                });
            }
        }

        stripe.write_raw(&data_buf);

        let mut repaired_indices: Vec<usize> = Vec::new();

        if let Some(restorer) = stripe.as_restore_mut() {
            let raid1_like = T::DATA == 1 && T::DISKS == D;

            if raid1_like {
                for &i in &missing_or_untrusted {
                    restorer.restore(i);
                    repaired_indices.push(i);
                }
            } else if missing_or_untrusted.len() == 1 {
                let i = missing_or_untrusted[0];
                restorer.restore(i);
                repaired_indices.push(i);
            }

            let scrub_rewrite = restorer.scrub();
            repaired_indices.extend(scrub_rewrite);
        }

        if !repaired_indices.is_empty() {
            repaired_indices.sort_unstable();
            repaired_indices.dedup();

            let mut raw: [Bits<N>; D] = [Bits::zero(); D];
            stripe.read_raw(&mut raw);

            for &i in &repaired_indices {
                if i >= D {
                    continue;
                }
                if self.0[i].is_missing() {
                    continue;
                }

                self.0[i].write_at(off, &raw[i].0);
            }
        }
    }
}
