use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::{align_of, size_of};

#[test]
fn zero_works_for_various_sizes() {
    let a1 = Bits::<1>::zero();
    assert_eq!(a1.as_bytes(), &[0u8; 1]);

    let a4 = Bits::<4>::zero();
    assert_eq!(a4.as_bytes(), &[0u8; 4]);

    let a32 = Bits::<32>::zero();
    assert_eq!(a32.as_bytes(), &[0u8; 32]);
}

#[test]
fn size_and_alignment_match_transparent_representation() {
    assert_eq!(size_of::<Bits<1>>(), size_of::<[u8; 1]>());
    assert_eq!(align_of::<Bits<1>>(), align_of::<[u8; 1]>());

    assert_eq!(size_of::<Bits<7>>(), size_of::<[u8; 7]>());
    assert_eq!(align_of::<Bits<7>>(), align_of::<[u8; 7]>());

    assert_eq!(size_of::<Bits<64>>(), size_of::<[u8; 64]>());
    assert_eq!(align_of::<Bits<64>>(), align_of::<[u8; 64]>());
}

#[test]
fn as_bytes_and_as_bytes_mut_expose_backing_storage() {
    let mut a = Bits::<4>::zero();
    assert_eq!(a.as_bytes(), &[0, 0, 0, 0]);

    let raw = a.as_bytes_mut();
    raw[1] = 0xAB;
    raw[3] = 0xCD;
    assert_eq!(a.as_bytes(), &[0, 0xAB, 0, 0xCD]);
}

#[test]
fn get_set_roundtrip_and_bit_order() {
    let mut a = Bits::<2>::zero();

    a.set(0, true);
    assert!(a.get(0));
    assert_eq!(a.as_bytes()[0], 0b0000_0001);

    a.set(7, true);
    assert!(a.get(7));
    assert_eq!(a.as_bytes()[0], 0b1000_0001);

    a.set(8, true);
    assert!(a.get(8));
    assert_eq!(a.as_bytes()[1], 0b0000_0001);

    a.set(7, false);
    assert!(!a.get(7));
    assert_eq!(a.as_bytes()[0], 0b0000_0001);
}

#[test]
fn xor_owned_and_assign_variants() {
    let left = Bits::<4>([0xFF, 0x00, 0xAA, 0x55]);
    let right = Bits::<4>([0x0F, 0x0F, 0xF0, 0xF0]);
    let expected = Bits::<4>([0xF0, 0x0F, 0x5A, 0xA5]);

    let combined = left ^ right;
    assert_eq!(combined.as_bytes(), expected.as_bytes());

    let mut accumulator = Bits::<4>([0xFF, 0x00, 0xAA, 0x55]);
    accumulator ^= Bits::<4>([0x0F, 0x0F, 0xF0, 0xF0]);
    assert_eq!(accumulator.as_bytes(), expected.as_bytes());

    let input = Bits::<4>([0x12, 0x34, 0x56, 0x78]);
    let mask = Bits::<4>([0xFF, 0xFF, 0x00, 0x00]);
    let masked = input ^ mask;
    assert_eq!(masked.as_bytes(), &[0xED, 0xCB, 0x56, 0x78]);

    let mut mutable = Bits::<4>([0x12, 0x34, 0x56, 0x78]);
    mutable ^= &mask;
    assert_eq!(mutable.as_bytes(), &[0xED, 0xCB, 0x56, 0x78]);

    let mut self_xor = Bits::<4>([1, 2, 3, 4]);
    let self_xor_clone = self_xor;
    self_xor ^= self_xor_clone;
    assert_eq!(self_xor.as_bytes(), &[0, 0, 0, 0]);
}

#[test]
fn xor_is_associative_and_commutative() {
    let left = Bits::<8>([0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]);
    let right = Bits::<8>([0xFF, 0x00, 0xFF, 0x00, 0xAA, 0x55, 0xAA, 0x55]);
    let third = Bits::<8>([0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80]);

    let assoc_left = (left ^ right) ^ third;
    let assoc_right = left ^ (right ^ third);
    assert_eq!(assoc_left, assoc_right);

    let left_then_right = left ^ right;
    let right_then_left = right ^ left;
    assert_eq!(left_then_right, right_then_left);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn get_panics_out_of_bounds() {
    let a = Bits::<1>::zero();
    let _ = a.get(8);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn set_panics_out_of_bounds() {
    let mut a = Bits::<2>::zero();
    a.set(16, true);
}

#[test]
fn hashing_equal_vals_produces_equal_hashes() {
    let a = Bits::<3>([1, 2, 3]);
    let b = Bits::<3>([1, 2, 3]);
    let c = Bits::<3>([3, 2, 1]);

    let mut ha = DefaultHasher::new();
    a.hash(&mut ha);
    let ha = ha.finish();

    let mut hb = DefaultHasher::new();
    b.hash(&mut hb);
    let hb = hb.finish();

    assert_eq!(a, b);
    assert_eq!(ha, hb);

    assert_ne!(a, c);
}

#[test]
fn zero_bits_all_false() {
    let z = Bits::<3>::zero();
    for i in 0..24 {
        assert!(!z.get(i));
    }
}
