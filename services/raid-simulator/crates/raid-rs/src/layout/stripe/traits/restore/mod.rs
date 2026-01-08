pub trait Restore {
    fn restore(&mut self, i: usize);

    fn scrub(&mut self) -> Vec<usize> {
        Vec::new()
    }
}
