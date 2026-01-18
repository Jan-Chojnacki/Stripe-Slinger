//! Fixed-width byte buffers with bit-level helpers for RAID layouts.

use std::ops::{BitXor, BitXorAssign};

#[cfg(test)]
mod bits_tests;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
/// Bits stores a fixed-size array of bytes with bitwise helpers.
pub struct Bits<const N: usize>(pub [u8; N]);

impl<const N: usize> Bits<N> {
    #[inline]
    #[must_use]
    /// `zero` returns a zero-initialized bit buffer.
    pub const fn zero() -> Self {
        Self([0u8; N])
    }
    #[inline]
    #[must_use]
    /// `as_bytes` returns a shared reference to the underlying byte array.
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }
    #[inline]
    /// `as_bytes_mut` returns a mutable reference to the underlying byte array.
    pub const fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
    }

    #[inline]
    #[must_use]
    /// `get` returns the bit value at the provided index.
    ///
    /// # Arguments
    /// * `i` - The bit index within the buffer.
    pub const fn get(&self, i: usize) -> bool {
        let (byte, bit) = (i >> 3, i & 7);
        (self.0[byte] >> bit) & 1 == 1
    }

    #[inline]
    /// `set` updates the bit at the provided index.
    ///
    /// # Arguments
    /// * `i` - The bit index within the buffer.
    /// * `val` - Whether the bit should be set.
    pub const fn set(&mut self, i: usize, val: bool) {
        let (byte, bit) = (i >> 3, i & 7);
        let m = 1u8 << bit;
        if val {
            self.0[byte] |= m;
        } else {
            self.0[byte] &= !m;
        }
    }

    #[inline]
    /// `xor_in_place` performs an in-place XOR with another buffer.
    ///
    /// # Arguments
    /// * `rhs` - The buffer to XOR into this one.
    pub fn xor_in_place(&mut self, rhs: &Self) {
        for (a, b) in self.0.iter_mut().zip(rhs.0.iter()) {
            *a ^= *b;
        }
    }
}

impl<const N: usize> BitXor for Bits<N> {
    type Output = Self;
    #[inline]
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self.xor_in_place(&rhs);
        self
    }
}

impl<const N: usize> BitXorAssign for Bits<N> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.xor_in_place(&rhs);
    }
}

impl<const N: usize> BitXor<&Self> for Bits<N> {
    type Output = Self;
    #[inline]
    fn bitxor(mut self, rhs: &Self) -> Self::Output {
        self.xor_in_place(rhs);
        self
    }
}

impl<const N: usize> BitXorAssign<&Self> for Bits<N> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &Self) {
        self.xor_in_place(rhs);
    }
}
