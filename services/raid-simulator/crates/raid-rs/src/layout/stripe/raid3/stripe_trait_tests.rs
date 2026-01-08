use crate::layout::bits::Bits;
use crate::layout::stripe::raid3::RAID3;
use crate::layout::stripe::traits::stripe::Stripe;

#[test]
fn stripe_data_const_matches_d_minus_one() {
    const DATA: usize = <RAID3<4, 4> as Stripe<4, 4>>::DATA;
    assert_eq!(DATA, 3);
}

#[test]
fn stripe_write_sets_data_and_parity_then_read_returns_same() {
    let d0 = Bits::<4>([1, 2, 3, 4]);
    let d1 = Bits::<4>([5, 6, 7, 8]);
    let d2 = Bits::<4>([9, 10, 11, 12]);

    let mut r = RAID3::<4, 4>([Bits::zero(); 4]);

    r.write(&[d0, d1, d2]);

    assert_eq!(r.0[0], d0);
    assert_eq!(r.0[1], d1);
    assert_eq!(r.0[2], d2);

    let mut expected_p = Bits::<4>::zero();
    expected_p ^= d0;
    expected_p ^= d1;
    expected_p ^= d2;
    assert_eq!(r.0[RAID3::<4, 4>::PARITY_IDX], expected_p);

    let mut out = [Bits::<4>::zero(); <RAID3<4, 4> as Stripe<4, 4>>::DATA];
    r.read(&mut out);
    assert_eq!(out, [d0, d1, d2]);
}

#[test]
fn stripe_write_raw_and_read_raw_cover_all_drives() {
    let values = [
        Bits::<2>([0x01, 0x02]),
        Bits::<2>([0x03, 0x04]),
        Bits::<2>([0x05, 0x06]),
        Bits::<2>([0x07, 0x08]),
    ];
    let mut r = RAID3::<4, 2>([Bits::zero(); 4]);

    r.write_raw(&values);

    assert_eq!(r.0, values);

    let mut out = [Bits::<2>::zero(); <RAID3<4, 2> as Stripe<4, 2>>::DISKS];
    r.read_raw(&mut out);

    assert_eq!(out, values);
}

#[test]
#[should_panic(expected = "RAID3 expects 2 chunks.")]
fn stripe_write_panics_on_wrong_len() {
    let d0 = Bits::<2>([0xAA, 0x55]);
    let mut r = RAID3::<3, 2>([Bits::zero(); 3]);
    r.write(&[d0][..1]);
}

#[test]
#[should_panic(expected = "Output buffer must be 2 chunks.")]
fn stripe_read_panics_on_wrong_out_len() {
    let d0 = Bits::<2>([1, 2]);
    let d1 = Bits::<2>([3, 4]);
    let mut r = RAID3::<3, 2>([Bits::zero(); 3]);

    r.write(&[d0, d1]);

    let mut out = [Bits::<2>::zero(); 1];
    r.read(&mut out);
}

#[test]
#[should_panic(expected = "RAID0 expects 4 chunks.")]
fn stripe_write_raw_panics_on_wrong_len() {
    let mut r = RAID3::<4, 2>([Bits::zero(); 4]);
    let values = [Bits::<2>::zero(); <RAID3<4, 2> as Stripe<4, 2>>::DISKS];
    r.write_raw(&values[..3]);
}

#[test]
#[should_panic(expected = "Output buffer must be 4 chunks.")]
fn stripe_read_raw_panics_on_wrong_out_len() {
    let values = [
        Bits::<2>([1, 2]),
        Bits::<2>([3, 4]),
        Bits::<2>([5, 6]),
        Bits::<2>([7, 8]),
    ];
    let mut r = RAID3::<4, 2>([Bits::zero(); 4]);
    r.write_raw(&values);

    let mut out = [Bits::<2>::zero(); 3];
    r.read_raw(&mut out);
}

#[test]
fn stripe_as_restore_returns_some() {
    let r = RAID3::<3, 4>([Bits::zero(); 3]);
    assert!(r.as_restore().is_some());
}
