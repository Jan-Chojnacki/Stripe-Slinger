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
#[should_panic]
fn stripe_write_panics_on_wrong_len() {
    let mut r = RAID0::<2, 2>::zero();
    r.write(&[Bits::<2>::zero()]);
}

#[test]
#[should_panic]
fn stripe_read_panics_on_wrong_out_len() {
    let values = [Bits::<1>([1]), Bits::<1>([2])];
    let mut r = RAID0::<2, 1>::zero();
    r.write(&values);

    let mut out = [Bits::<1>::zero(); 1];
    r.read(&mut out);
}

#[test]
fn stripe_as_restore_returns_none() {
    let r = RAID0::<2, 4>::zero();
    assert!(r.as_restore().is_none());
}
