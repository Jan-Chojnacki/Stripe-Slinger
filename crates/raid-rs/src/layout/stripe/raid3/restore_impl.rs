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
}
