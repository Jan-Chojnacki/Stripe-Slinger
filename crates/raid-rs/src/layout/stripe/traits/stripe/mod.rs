#[cfg(test)]
mod stripe_tests;

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::restore::Restore;

pub trait Stripe<const D: usize, const N: usize> {
    const DATA: usize;
    const DISKS: usize;

    fn write(&mut self, data: &[Bits<N>]);
    fn write_raw(&mut self, data: &[Bits<N>]);
    fn read(&self, out: &mut [Bits<N>]);
    fn read_raw(&self, out: &mut [Bits<N>]);
    fn as_restore(&self) -> Option<&dyn Restore> {
        None
    }
}
