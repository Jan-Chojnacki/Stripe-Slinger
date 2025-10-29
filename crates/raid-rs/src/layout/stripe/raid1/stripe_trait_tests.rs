use crate::layout::bits::Bits;
use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::stripe::Stripe;

#[test]
fn stripe_data_const_is_one() {
    const DATA: usize = <RAID1<2, 4> as Stripe<4>>::DATA;
    assert_eq!(DATA, 1);
}

#[test]
fn stripe_write_mirrors_across_all_drives_then_read_returns_value() {
    let value = Bits::<4>([1, 2, 3, 4]);
    let mut r = RAID1::<3, 4>([Bits::zero(); 3]);

    r.write(&[value]);

    for drive in r.0.iter() {
        assert_eq!(*drive, value);
    }

    let mut out = [Bits::<4>::zero(); <RAID1<3, 4> as Stripe<4>>::DATA];
    r.read(&mut out);
    assert_eq!(out, [value]);
}

#[test]
#[should_panic]
fn stripe_write_panics_on_wrong_len() {
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    r.write(&[]);
}

#[test]
#[should_panic]
fn stripe_read_panics_on_wrong_out_len() {
    let value = Bits::<2>([0xAA, 0x55]);
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    r.write(&[value]);

    let mut out = [Bits::<2>::zero(); 0];
    r.read(&mut out);
}

#[test]
fn stripe_as_restore_returns_some() {
    let r = RAID1::<2, 4>([Bits::zero(); 2]);
    assert!(r.as_restore().is_some());
}
