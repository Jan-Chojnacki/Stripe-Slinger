use crate::layout::bits::Bits;
use crate::layout::stripe::raid3::RAID3;

#[test]
fn zero_initializes_all_drives() {
    let r = RAID3::<3, 4>::zero();
    for d in 0..3 {
        assert_eq!(r.0[d].as_bytes(), &[0u8; 4], "drive {d}");
    }
    assert_eq!(RAID3::<3, 4>::PARITY_IDX, 2);
}

#[test]
fn write_parity_basic_and_idempotent() {
    let d0 = Bits::<4>([0xFF, 0x00, 0xAA, 0x55]);
    let d1 = Bits::<4>([0x0F, 0xF0, 0xF0, 0x0F]);
    let mut r = RAID3::<3, 4>([d0, d1, Bits::zero()]);

    r.write_parity();

    let mut expected = Bits::<4>::zero();
    expected ^= d0;
    expected ^= d1;
    assert_eq!(r.0[RAID3::<3, 4>::PARITY_IDX], expected);

    let before = r.0[RAID3::<3, 4>::PARITY_IDX];
    r.write_parity();
    assert_eq!(r.0[RAID3::<3, 4>::PARITY_IDX], before);

    let mut acc = Bits::<4>::zero();
    for b in &r.0 {
        acc ^= *b;
    }
    assert_eq!(acc.as_bytes(), &[0u8; 4]);
}

#[test]
fn reconstruct_in_place_recovers_original() {
    let d0 = Bits::<4>([1, 2, 3, 4]);
    let d1 = Bits::<4>([5, 6, 7, 8]);
    let d2 = Bits::<4>([9, 10, 11, 12]);
    let expected = [d0, d1, d2];

    for i in 0..RAID3::<4, 4>::PARITY_IDX {
        let mut r = RAID3::<4, 4>([d0, d1, d2, Bits::zero()]);

        r.write_parity();
        r.0[i] = Bits::zero();
        r.reconstruct_data(i);

        for (j, expected_chunk) in expected.iter().enumerate().take(RAID3::<4, 4>::PARITY_IDX) {
            assert_eq!(r.0[j], *expected_chunk);
        }
    }
}
