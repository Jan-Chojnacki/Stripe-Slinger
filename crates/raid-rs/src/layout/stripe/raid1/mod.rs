use crate::layout::bits::Bits;

#[cfg(test)]
mod raid1_tests;
mod restore_impl;
#[cfg(test)]
mod restore_trait_tests;
mod stripe_impl;
#[cfg(test)]
mod stripe_trait_tests;

pub struct RAID1<const D: usize, const N: usize>(pub [Bits<N>; D]);

impl<const D: usize, const N: usize> RAID1<D, N> {
    pub const fn zero() -> Self {
        Self([Bits::<N>::zero(); D])
    }

    fn copy_from(&mut self, src: usize, dst: usize) {
        self.0[dst] = self.0[src];
    }
}
