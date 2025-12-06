use crate::layout::bits::Bits;
use crate::layout::stripe::raid3::RAID3;
use crate::layout::stripe::traits::restore::Restore;

#[test]
fn restore_recovers_missing_data_drive() {
    let d0 = Bits::<4>([10, 20, 30, 40]);
    let d1 = Bits::<4>([1, 2, 3, 4]);
    let d2 = Bits::<4>([7, 8, 9, 10]);
    let expected = [d0, d1, d2];

    for missing in 0..RAID3::<4, 4>::PARITY_IDX {
        let mut r = RAID3::<4, 4>([d0, d1, d2, Bits::zero()]);
        r.write_parity();

        r.0[missing] = Bits::zero();

        let restorer: &mut dyn Restore = &mut r;
        restorer.restore(missing);

        for i in 0..RAID3::<4, 4>::PARITY_IDX {
            assert_eq!(r.0[i], expected[i]);
        }
    }
}

#[test]
fn restore_recomputes_parity_when_parity_corrupted() {
    let d0 = Bits::<2>([0xAA, 0x55]);
    let d1 = Bits::<2>([0x0F, 0xF0]);
    let d2 = Bits::<2>([0xFF, 0x00]);

    let mut r = RAID3::<4, 2>([d0, d1, d2, Bits::zero()]);
    r.write_parity();

    r.0[RAID3::<4, 2>::PARITY_IDX] = Bits::<2>([0xDE, 0xAD]);

    let restorer: &mut dyn Restore = &mut r;
    restorer.restore(RAID3::<4, 2>::PARITY_IDX);

    let mut expected_p = Bits::<2>::zero();
    for i in 0..RAID3::<4, 2>::PARITY_IDX {
        expected_p ^= r.0[i];
    }
    assert_eq!(r.0[RAID3::<4, 2>::PARITY_IDX], expected_p);

    let mut acc = Bits::<2>::zero();
    for b in r.0.iter() {
        acc ^= *b;
    }
    assert_eq!(acc.as_bytes(), &[0u8; 2]);
}

#[test]
#[should_panic]
fn restore_panics_on_invalid_index() {
    let d0 = Bits::<1>([1]);
    let d1 = Bits::<1>([2]);

    let mut r = RAID3::<3, 1>([d0, d1, Bits::zero()]);
    r.write_parity();

    let invalid = RAID3::<3, 1>::PARITY_IDX + 1;
    let restorer: &mut dyn Restore = &mut r;
    restorer.restore(invalid);
}
