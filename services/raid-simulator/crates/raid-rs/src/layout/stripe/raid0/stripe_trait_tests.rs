use crate::layout::bits::Bits;
use crate::layout::stripe::raid0::RAID0;
use crate::layout::stripe::traits::stripe::Stripe;

#[test]
fn stripe_data_const_matches_drive_count() {
    const DATA: usize = <RAID0<3, 4> as Stripe<3, 4>>::DATA;
    assert_eq!(DATA, 3);
}

#[test]
fn stripe_write_and_read_cover_all_drives() {
    let values = [Bits::<2>([0xAA, 0x55]), Bits::<2>([0x0F, 0xF0])];
    let mut r = RAID0::<2, 2>::zero();

    r.write(&values);

    let mut out = [Bits::<2>::zero(); <RAID0<2, 2> as Stripe<2, 2>>::DATA];
    r.read(&mut out);

    assert_eq!(out, values);
}

#[test]
fn stripe_write_raw_and_read_raw_cover_all_drives() {
    let values = [
        Bits::<2>([0x01, 0x02]),
        Bits::<2>([0x03, 0x04]),
        Bits::<2>([0x05, 0x06]),
    ];
    let mut r = RAID0::<3, 2>::zero();

    r.write_raw(&values);

    assert_eq!(r.0, values);

    let mut out = [Bits::<2>::zero(); <RAID0<3, 2> as Stripe<3, 2>>::DISKS];
    r.read_raw(&mut out);

    assert_eq!(out, values);
}

#[test]
#[should_panic(expected = "RAID0 expects 2 chunks.")]
fn stripe_write_panics_on_wrong_len() {
    let mut r = RAID0::<2, 2>::zero();
    r.write(&[Bits::<2>::zero()]);
}

#[test]
#[should_panic(expected = "RAID0 expects 2 chunks.")]
fn stripe_write_raw_panics_on_wrong_len() {
    let mut r = RAID0::<2, 2>::zero();
    let values = [Bits::<2>::zero(); <RAID0<2, 2> as Stripe<2, 2>>::DISKS];
    r.write_raw(&values[..1]);
}

#[test]
#[should_panic(expected = "Output buffer must be 2 chunks.")]
fn stripe_read_panics_on_wrong_out_len() {
    let values = [Bits::<1>([1]), Bits::<1>([2])];
    let mut r = RAID0::<2, 1>::zero();
    r.write(&values);

    let mut out = [Bits::<1>::zero(); 1];
    r.read(&mut out);
}

#[test]
#[should_panic(expected = "Output buffer must be 3 chunks.")]
fn stripe_read_raw_panics_on_wrong_out_len() {
    let values = [Bits::<1>([1]), Bits::<1>([2]), Bits::<1>([3])];
    let mut r = RAID0::<3, 1>::zero();
    r.write_raw(&values);

    let mut out = [Bits::<1>::zero(); 2];
    r.read_raw(&mut out);
}

#[test]
fn stripe_as_restore_returns_none() {
    let r = RAID0::<2, 4>::zero();
    assert!(r.as_restore().is_none());
}
