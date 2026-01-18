use std::collections::HashMap;

use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::restore::Restore;

impl<const D: usize, const N: usize> Restore for RAID1<D, N> {
    fn restore(&mut self, i: usize) {
        assert!(i < D, "RAID1 have {D} disks, {i} is not valid index.");
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

    fn scrub(&mut self) -> Vec<usize> {
        let mut counts: HashMap<_, usize> = HashMap::new();
        for b in &self.0 {
            *counts.entry(*b).or_insert(0) += 1;
        }

        let mut best = self.0[0];
        let mut best_count = 0usize;
        for (val, c) in counts {
            if c > best_count {
                best = val;
                best_count = c;
            }
        }
        let mut to_rewrite = Vec::new();
        for i in 0..D {
            if self.0[i] != best {
                self.0[i] = best;
                to_rewrite.push(i);
            }
        }
        to_rewrite
    }
}
