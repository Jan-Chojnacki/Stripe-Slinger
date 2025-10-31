use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::retention::disk::Disk;

pub struct Array<const D: usize, const N: usize> (pub [Disk; D]);

impl<const D: usize, const N: usize> Array<D, N> {
    pub fn init_array(paths: [String; D]) -> Self {
        let array: [Disk; D] = std::array::from_fn(|i| {
            Disk::open_prealloc(&paths[i], 1024).unwrap()
        });

        Self { 0: array }
    }

    pub fn write<T: Stripe<D, N>>(&mut self, off: u64, stripe: &T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];
        stripe.read_raw(&mut data_buf);
        for i in 0..D {
            self.0[i].write_at(off, &data_buf[i].0);
        }
    }

    pub fn read<T: Stripe<D, N>>(&mut self, off: u64, stripe: &mut T) {
        let mut data_buf: [Bits<N>; D] = [Bits::zero(); D];
        for i in 0..D {
            self.0[i].read_at(off, &mut data_buf[i].0);
        }
        stripe.write_raw(&data_buf);
    }
}