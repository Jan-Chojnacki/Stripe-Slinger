use crate::layout::bits::Bits;
use crate::layout::stripe::raid0::RAID0;
use crate::layout::stripe::traits::stripe::Stripe;

impl<const D: usize, const N: usize> Stripe<D, N> for RAID0<D, N> {
    const DATA: usize = D;
    const DISKS: usize = D;

    fn write(&mut self, data: &[Bits<N>]) {
        assert_eq!(
            data.len(),
            Self::DATA,
            "RAID0 expects {} chunks.",
            Self::DATA
        );
        self.0[..Self::DATA].copy_from_slice(&data[..Self::DATA]);
    }

    fn write_raw(&mut self, data: &[Bits<N>]) {
        assert_eq!(
            data.len(),
            Self::DISKS,
            "RAID0 expects {} chunks.",
            Self::DISKS
        );
        self.0[..Self::DISKS].copy_from_slice(&data[..Self::DISKS]);
    }

    fn read(&self, out: &mut [Bits<N>]) {
        assert_eq!(
            out.len(),
            Self::DATA,
            "Output buffer must be {} chunks.",
            Self::DATA
        );
        out[..Self::DATA].copy_from_slice(&self.0[..Self::DATA]);
    }

    fn read_raw(&self, out: &mut [Bits<N>]) {
        assert_eq!(
            out.len(),
            Self::DISKS,
            "Output buffer must be {} chunks.",
            Self::DISKS
        );
        out[..Self::DISKS].copy_from_slice(&self.0[..Self::DISKS]);
    }
}
