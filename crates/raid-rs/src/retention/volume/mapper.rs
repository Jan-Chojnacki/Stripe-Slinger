use crate::layout::stripe::traits::stripe::Stripe;

pub struct Geometry {
    pub bytes_per_chunk: usize,
    pub bytes_per_stripe: usize,
}

pub fn geometry<const D: usize, const N: usize, S: Stripe<D, N>>() -> Geometry {
    Geometry {
        bytes_per_chunk: N,
        bytes_per_stripe: S::DATA * N,
    }
}

pub fn locate_byte(byte_offset: u64, byte_delta: usize, geom: &Geometry) -> (u64, usize) {
    let absolute = byte_offset
        .checked_add(byte_delta as u64)
        .expect("byte offset overflow");
    let stripe = absolute / geom.bytes_per_stripe as u64;
    let in_stripe = (absolute % geom.bytes_per_stripe as u64) as usize;
    (stripe, in_stripe)
}

pub fn stripe_byte_offset<const N: usize>(stripe_index: u64) -> u64 {
    stripe_index
        .checked_mul(N as u64)
        .expect("stripe offset overflow")
}