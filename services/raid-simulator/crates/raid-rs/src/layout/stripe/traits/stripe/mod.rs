//! Stripe trait definitions for reading and writing RAID layouts.

#[cfg(test)]
mod stripe_tests;

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::restore::Restore;

/// Stripe describes read/write behavior for a RAID stripe.
pub trait Stripe<const D: usize, const N: usize> {
    /// DATA is the number of data disks used in the stripe.
    const DATA: usize;
    /// DISKS is the total number of disks used by the stripe layout.
    const DISKS: usize;

    /// write encodes data into the stripe layout.
    ///
    /// # Arguments
    /// * `data` - The data blocks to encode into the stripe.
    fn write(&mut self, data: &[Bits<N>]);
    /// write_raw writes raw blocks into the stripe without parity calculations.
    ///
    /// # Arguments
    /// * `data` - The raw blocks to copy into the stripe.
    fn write_raw(&mut self, data: &[Bits<N>]);
    /// read decodes the stripe layout into data blocks.
    ///
    /// # Arguments
    /// * `out` - The output buffer to populate with decoded data blocks.
    fn read(&self, out: &mut [Bits<N>]);
    /// read_raw reads raw blocks from the stripe without decoding.
    ///
    /// # Arguments
    /// * `out` - The output buffer to populate with raw blocks.
    fn read_raw(&self, out: &mut [Bits<N>]);
    /// as_restore returns a restoration trait object if supported.
    fn as_restore(&self) -> Option<&dyn Restore> {
        None
    }
    /// as_restore_mut returns a mutable restoration trait object if supported.
    fn as_restore_mut(&mut self) -> Option<&mut dyn Restore> {
        None
    }
}
