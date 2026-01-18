//! Restoration helpers for rebuilding failed or stale disks.

/// Restore defines hooks for rebuilding missing or stale stripe members.
pub trait Restore {
    /// restore rebuilds the stripe member at the provided index.
    ///
    /// # Arguments
    /// * `i` - The disk index to rebuild.
    fn restore(&mut self, i: usize);

    /// scrub returns indices that should be rewritten after a read.
    ///
    /// # Returns
    /// A list of disk indices that require a rewrite.
    fn scrub(&mut self) -> Vec<usize> {
        Vec::new()
    }
}
