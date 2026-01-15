//! RAID1 stripe layout implementation.

use crate::layout::bits::Bits;

#[cfg(test)]
mod raid1_tests;
mod restore_impl;
#[cfg(test)]
mod restore_trait_tests;
mod stripe_impl;
#[cfg(test)]
mod stripe_trait_tests;

/// RAID1 stores mirrored copies of each data block.
pub struct RAID1<const D: usize, const N: usize>(pub [Bits<N>; D]);

impl<const D: usize, const N: usize> RAID1<D, N> {
    #[must_use]
    /// zero returns a zero-initialized RAID1 stripe.
    pub const fn zero() -> Self {
        Self([Bits::<N>::zero(); D])
    }

    const fn copy_from(&mut self, src: usize, dst: usize) {
        self.0[dst] = self.0[src];
    }
}
