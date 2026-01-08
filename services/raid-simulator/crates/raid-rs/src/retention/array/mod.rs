#[cfg(test)]
mod array_tests;

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::retention::disk::Disk;
use std::fmt::Write;

pub struct Array<const D: usize, const N: usize>(pub [Disk; D]);

impl<const D: usize, const N: usize> Array<D, N> {
    /// Creates an array with preallocated disk images.
    ///
    /// # Panics
    /// Panics if any disk image cannot be created or opened.
    #[must_use]
    pub fn init_array(paths: &[String; D], len: u64) -> Self {
        let array: [Disk; D] =
            std::array::from_fn(|i| Disk::open_prealloc(&paths[i], len).unwrap());

        Self(array)
    }

    #[must_use]
    pub fn disk_len(&self) -> u64 {
        self.0.first().map_or(0, Disk::len)
    }

    /// # Errors
    /// Returns an error if the disk index is out of range or the disk cannot be failed.
    pub fn fail_disk(&mut self, i: usize) -> anyhow::Result<()> {
        if i >= D {
            anyhow::bail!("disk index out of range: {i} (D={D})");
        }
        self.0[i].fail()
    }

    /// # Errors
    /// Returns an error if the disk index is out of range or the disk cannot be replaced.
    pub fn replace_disk(&mut self, i: usize) -> anyhow::Result<()> {
        if i >= D {
            anyhow::bail!("disk index out of range: {i} (D={D})");
        }
        self.0[i].replace()
    }

    #[must_use]
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

    pub fn write<T: Stripe<D, N>>(&mut self, off: u64, stripe: &T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];
        stripe.read_raw(&mut data_buf);

        for (disk, data) in self.0.iter_mut().zip(&data_buf) {
            if !disk.is_missing() {
                disk.write_at(off, &data.0);
            }
        }
    }

    pub fn read<T: Stripe<D, N>>(&mut self, off: u64, stripe: &mut T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];

        // Collect indices that are missing OR present-but-untrusted (needs rebuild).
        let mut missing_or_untrusted: Vec<usize> = Vec::new();

        for (i, (disk, data)) in self.0.iter_mut().zip(data_buf.iter_mut()).enumerate() {
            let disk_missing = disk.is_missing();
            let untrusted = disk.needs_rebuild;

            if disk_missing || untrusted {
                missing_or_untrusted.push(i);
                continue;
            }
            disk.read_at(off, &mut data.0);
        }

        stripe.write_raw(&data_buf);

        // Attempt restore when the layout supports it.
        let mut repaired_indices: Vec<usize> = Vec::new();

        if let Some(restorer) = stripe.as_restore_mut() {
            // Heuristic mode detection:
            // - RAID1-like: DATA == 1 (mirroring) -> can restore multiple disks.
            // - RAID3-like: DATA + 1 == DISKS (single parity) -> can restore only one missing.
            let raid1_like = T::DATA == 1 && T::DISKS == D;
            let raid3_like = T::DATA + 1 == T::DISKS && T::DISKS == D;

            if raid1_like {
                for &i in &missing_or_untrusted {
                    restorer.restore(i);
                    repaired_indices.push(i);
                }
            } else if raid3_like {
                if missing_or_untrusted.len() == 1 {
                    let i = missing_or_untrusted[0];
                    restorer.restore(i);
                    repaired_indices.push(i);
                }
            } else {
                // Unknown redundancy pattern: only try single-disk restore.
                if missing_or_untrusted.len() == 1 {
                    let i = missing_or_untrusted[0];
                    restorer.restore(i);
                    repaired_indices.push(i);
                }
            }

            // Scrub for inconsistencies (e.g. RAID1 mismatch or RAID3 parity mismatch).
            let scrub_rewrite = restorer.scrub();
            repaired_indices.extend(scrub_rewrite);
        }

        // Write-back repaired stripes (read-repair): if a disk is present (operational)
        // and either:
        // - it was "untrusted" (needs rebuild) and we reconstructed it,
        // - or scrub marked it for rewrite.
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
                // Write back repaired stripes to present disks.
                self.0[i].write_at(off, &raw[i].0);
            }
        }
    }
}
