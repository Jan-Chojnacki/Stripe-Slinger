pub trait Restore {
    /// Reconstruct missing data for the given disk index `i` inside the stripe.
    fn restore(&mut self, i: usize);

    /// Validate stripe consistency and optionally repair in-memory stripe contents.
    ///
    /// Returns a list of disk indices that should be rewritten with the repaired data.
    fn scrub(&mut self) -> Vec<usize> {
        Vec::new()
    }
}
