//! Geometry helpers for mapping logical byte offsets to stripes.

use crate::layout::stripe::traits::stripe::Stripe;

/// Geometry describes the byte layout of stripes and chunks.
pub struct Geometry {
    pub bytes_per_chunk: usize,
    pub bytes_per_stripe: usize,
}

/// `geometry` computes the stripe geometry for a given layout.
///
/// # Returns
/// A Geometry describing chunk and stripe sizes in bytes.
pub fn geometry<const D: usize, const N: usize, S: Stripe<D, N>>() -> Geometry {
    Geometry {
        bytes_per_chunk: N,
        bytes_per_stripe: S::DATA * N,
    }
}

/// `locate_byte` maps a logical byte offset to its stripe index and in-stripe offset.
///
/// # Arguments
/// * `byte_offset` - Starting logical byte offset.
/// * `byte_delta` - Additional byte offset within the requested operation.
/// * `geom` - Geometry describing stripe sizing.
///
/// # Returns
/// A tuple of `(stripe_index, in_stripe_offset)`.
///
/// # Panics
/// Panics if the resulting byte offset overflows.
#[allow(clippy::missing_const_for_fn)]
pub fn locate_byte(byte_offset: u64, byte_delta: usize, geom: &Geometry) -> (u64, usize) {
    let absolute = byte_offset
        .checked_add(byte_delta as u64)
        .expect("byte offset overflow");
    let stripe = absolute / geom.bytes_per_stripe as u64;
    let in_stripe = usize::try_from(absolute % geom.bytes_per_stripe as u64)
        .expect("stripe offset exceeds usize");
    (stripe, in_stripe)
}

/// `stripe_byte_offset` returns the byte offset for the start of a stripe.
///
/// # Arguments
/// * `stripe_index` - Index of the stripe within the volume.
///
/// # Returns
/// The byte offset for the stripe start.
///
/// # Panics
/// Panics if the offset calculation overflows.
pub fn stripe_byte_offset<const N: usize>(stripe_index: u64) -> u64 {
    stripe_index
        .checked_mul(N as u64)
        .expect("stripe offset overflow")
}
