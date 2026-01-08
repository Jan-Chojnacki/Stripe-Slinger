use crate::layout::bits::Bits;
use crate::layout::stripe::raid3::RAID3;
use crate::layout::stripe::traits::restore::Restore;

impl<const D: usize, const N: usize> Restore for RAID3<D, N> {
    fn restore(&mut self, i: usize) {
        if i == Self::PARITY_IDX {
            self.write_parity();
        } else {
            self.reconstruct_data(i);
        }
    }

    fn scrub(&mut self) -> Vec<usize> {
        // Validate parity against data disks. If parity is wrong, recompute and mark it for rewrite.
        let mut p = Bits::<N>::zero();
        for i in 0..Self::PARITY_IDX {
            p ^= self.0[i];
        }
        if self.0[Self::PARITY_IDX] == p {
            Vec::new()
        } else {
            self.0[Self::PARITY_IDX] = p;
            vec![Self::PARITY_IDX]
        }
    }
}
