use crate::layout::bits::Bits;
use crate::layout::stripe::raid3::RAID3;
use crate::layout::stripe::traits::restore::Restore;
use crate::layout::stripe::traits::stripe::Stripe;

impl<const D: usize, const N: usize> Stripe<D, N> for RAID3<D, N> {
    const DATA: usize = D - 1;
    const DISKS: usize = D;

    fn write(&mut self, data: &[Bits<N>]) {
        assert_eq!(
            data.len(),
            Self::DATA,
            "RAID3 expects {} chunks.",
            Self::DATA
        );
        self.0[..Self::DATA].copy_from_slice(&data[..Self::DATA]);
        self.write_parity();
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

    fn as_restore(&self) -> Option<&dyn Restore> {
        Some(self)
    }

    fn as_restore_mut(&mut self) -> Option<&mut dyn Restore> {
        Some(self)
    }
}
