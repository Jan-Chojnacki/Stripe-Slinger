use std::ops::{BitXor, BitXorAssign};

#[cfg(test)]
mod bits_tests;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct Bits<const N: usize>(pub [u8; N]);

impl<const N: usize> Bits<N> {
    #[inline]
    #[must_use]
    pub const fn zero() -> Self {
        Self([0u8; N])
    }
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }
    #[inline]
    pub const fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
    }

    #[inline]
    #[must_use]
    pub const fn get(&self, i: usize) -> bool {
        let (byte, bit) = (i >> 3, i & 7);
        (self.0[byte] >> bit) & 1 == 1
    }

    #[inline]
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
