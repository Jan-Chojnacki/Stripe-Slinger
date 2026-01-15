//! RAID0 stripe layout implementation.

use crate::layout::bits::Bits;

#[cfg(test)]
mod raid0_tests;
mod stripe_impl;
#[cfg(test)]
mod stripe_trait_tests;

/// RAID0 stores raw striped blocks without parity.
pub struct RAID0<const D: usize, const N: usize>(pub [Bits<N>; D]);

impl<const D: usize, const N: usize> RAID0<D, N> {
    #[must_use]
    /// zero returns a zero-initialized RAID0 stripe.
    pub const fn zero() -> Self {
        Self([Bits::<N>::zero(); D])
    }
}
