use crate::layout::bits::Bits;

#[cfg(test)]
mod raid3_tests;
mod restore_impl;
#[cfg(test)]
mod restore_trait_tests;
mod stripe_impl;
#[cfg(test)]
mod stripe_trait_tests;

pub struct RAID3<const D: usize, const N: usize>(pub [Bits<N>; D]);

impl<const D: usize, const N: usize> RAID3<D, N> {
    const PARITY_IDX: usize = D - 1;

    pub const fn zero() -> Self {
        Self([Bits::<N>::zero(); D])
    }

    fn write_parity(&mut self) {
        let mut p = Bits::<N>::zero();
        for i in 0..Self::PARITY_IDX {
            p ^= self.0[i];
        }
        self.0[Self::PARITY_IDX] = p;
    }

    fn reconstruct_data(&mut self, i: usize) {
        assert!(i < D, "RAID3 have {} disks, {} is not valid index.", D, i);
        let mut acc = self.0[Self::PARITY_IDX];
        for j in 0..Self::PARITY_IDX {
            if j != i {
                acc ^= self.0[j];
            }
        }
        self.0[i] = acc;
    }
}
