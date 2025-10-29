use crate::layout::bits::Bits;
use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::restore::Restore;
use crate::layout::stripe::traits::stripe::Stripe;

impl<const D: usize, const N: usize> Stripe<N> for RAID1<D, N> {
    const DATA: usize = 1;

    fn write(&mut self, data: &[Bits<N>]) {
        assert_eq!(
            data.len(),
            Self::DATA,
            "RAID1 expects {} chunk.",
            Self::DATA
        );
        let value = data[0];
        for drive in self.0.iter_mut() {
            *drive = value;
        }
    }

    fn read(&self, out: &mut [Bits<N>]) {
        assert_eq!(
            out.len(),
            Self::DATA,
            "Output buffer must be {} chunk.",
            Self::DATA
        );
        if D > 0 {
            out[0] = self.0[0];
        }
    }

    fn as_restore(&self) -> Option<&dyn Restore> {
        Some(self)
    }
}
