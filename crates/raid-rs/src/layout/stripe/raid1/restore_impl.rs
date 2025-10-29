use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::restore::Restore;

impl<const D: usize, const N: usize> Restore for RAID1<D, N> {
    fn restore(&mut self, i: usize) {
        assert!(i < D, "RAID1 have {} disks, {} is not valid index.", D, i);
        let mut source = None;
        for j in 0..D {
            if j != i {
                source = Some(j);
                break;
            }
        }
        match source {
            Some(src) => {
                self.copy_from(src, i);
            }
            None => panic!("RAID1 requires at least two drives to restore"),
        }
    }
}
