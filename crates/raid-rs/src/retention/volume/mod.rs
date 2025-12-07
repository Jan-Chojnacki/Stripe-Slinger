mod mapper;
#[cfg(test)]
mod mapper_tests;
#[cfg(test)]
mod volume_tests;

use mapper::{Geometry, geometry, locate_byte, stripe_byte_offset};

use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use crate::retention::array::Array;

pub struct Volume<const D: usize, const N: usize, T: Stripe<D, N>> {
    array: Array<D, N>,
    layout: T,
    geom: Geometry,
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> Volume<D, N, T> {
    pub fn new(array: Array<D, N>, layout: T) -> Self {
        Self {
            array,
            geom: geometry::<D, N, T>(),
            layout,
        }
    }

    pub fn write_bytes(&mut self, byte_offset: u64, payload: &[u8]) {
        let mut data_chunks = vec![Bits::<N>::zero(); T::DATA];

        let mut written: usize = 0;
        let total = payload.len();
        while written < total {
            let (stripe_index, in_stripe_byte) = locate_byte(byte_offset, written, &self.geom);
            let stripe_bytes = self.geom.bytes_per_stripe - in_stripe_byte;
            let take = stripe_bytes.min(total - written);

            self.load_stripe(stripe_index);

            self.layout.read(&mut data_chunks);

            for i in 0..take {
                let byte_in_stripe = in_stripe_byte + i;
                let chunk_index = byte_in_stripe / self.geom.bytes_per_chunk;
                let byte_index = byte_in_stripe % self.geom.bytes_per_chunk;
                data_chunks[chunk_index].as_bytes_mut()[byte_index] = payload[written + i];
            }

            self.layout.write(&data_chunks);
            self.store_stripe(stripe_index);
            written += take;
        }
    }

    pub fn read_bytes(&mut self, byte_offset: u64, out: &mut [u8]) {
        let mut data_chunks = vec![Bits::<N>::zero(); T::DATA];

        let mut read: usize = 0;
        let total = out.len();
        while read < total {
            let (stripe_index, in_stripe_byte) = locate_byte(byte_offset, read, &self.geom);
            let stripe_bytes = self.geom.bytes_per_stripe - in_stripe_byte;
            let take = stripe_bytes.min(total - read);

            self.load_stripe(stripe_index);

            self.layout.read(&mut data_chunks);

            for i in 0..take {
                let byte_in_stripe = in_stripe_byte + i;
                let chunk_index = byte_in_stripe / self.geom.bytes_per_chunk;
                let byte_index = byte_in_stripe % self.geom.bytes_per_chunk;
                out[read + i] = data_chunks[chunk_index].as_bytes()[byte_index];
            }

            read += take;
        }
    }

    fn load_stripe(&mut self, stripe_index: u64) {
        let byte_offset = stripe_byte_offset::<N>(stripe_index);
        self.array.read(byte_offset, &mut self.layout);
    }

    fn store_stripe(&mut self, stripe_index: u64) {
        let byte_offset = stripe_byte_offset::<N>(stripe_index);
        self.array.write(byte_offset, &self.layout);
    }
}
