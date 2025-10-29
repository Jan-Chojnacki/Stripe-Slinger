use crate::layout::bits::Bits;
use crate::layout::stripe::raid0::RAID0;
use crate::layout::stripe::traits::stripe::Stripe;

impl<const D: usize, const N: usize> Stripe<N> for RAID0<D, N> {
    const DATA: usize = D;

    fn write(&mut self, data: &[Bits<N>]) {
        assert_eq!(
            data.len(),
            Self::DATA,
            "RAID0 expects {} chunks.",
            Self::DATA
        );
        for i in 0..Self::DATA {
            self.0[i] = data[i];
        }
    }

    fn read(&self, out: &mut [Bits<N>]) {
        assert_eq!(
            out.len(),
            Self::DATA,
            "Output buffer must be {} chunks.",
            Self::DATA
        );
        for i in 0..Self::DATA {
            out[i] = self.0[i];
        }
    }
}
