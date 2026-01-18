use crate::layout::bits::Bits;
use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::stripe::Stripe;

#[test]
fn stripe_data_const_is_one() {
    const DATA: usize = <RAID1<2, 4> as Stripe<2, 4>>::DATA;
    assert_eq!(DATA, 1);
}

#[test]
fn stripe_write_mirrors_across_all_drives_then_read_returns_value() {
    let value = Bits::<4>([1, 2, 3, 4]);
    let mut r = RAID1::<3, 4>([Bits::zero(); 3]);

    r.write(&[value]);

    for drive in &r.0 {
        assert_eq!(*drive, value);
    }

    let mut out = [Bits::<4>::zero(); <RAID1<3, 4> as Stripe<3, 4>>::DATA];
    r.read(&mut out);
    assert_eq!(out, [value]);
}

#[test]
fn stripe_write_raw_and_read_raw_cover_all_drives() {
    let values = [
        Bits::<2>([0x01, 0x02]),
        Bits::<2>([0x03, 0x04]),
        Bits::<2>([0x05, 0x06]),
    ];
    let mut r = RAID1::<3, 2>([Bits::zero(); 3]);

    r.write_raw(&values);

    assert_eq!(r.0, values);

    let mut out = [Bits::<2>::zero(); <RAID1<3, 2> as Stripe<3, 2>>::DISKS];
    r.read_raw(&mut out);

    assert_eq!(out, values);
}

#[test]
#[should_panic(expected = "RAID1 expects 1 chunk.")]
fn stripe_write_panics_on_wrong_len() {
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    r.write(&[]);
}

#[test]
#[should_panic(expected = "RAID0 expects 2 chunks.")]
fn stripe_write_raw_panics_on_wrong_len() {
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    let values = [Bits::<2>::zero(); <RAID1<2, 2> as Stripe<2, 2>>::DISKS];
    r.write_raw(&values[..1]);
}

#[test]
#[should_panic(expected = "Output buffer must be 1 chunk.")]
fn stripe_read_panics_on_wrong_out_len() {
    let value = Bits::<2>([0xAA, 0x55]);
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    r.write(&[value]);

    #[allow(clippy::zero_repeat_side_effects)]
    let mut out = [Bits::<2>::zero(); 0];
    r.read(&mut out);
}

#[test]
#[should_panic(expected = "Output buffer must be 2 chunks.")]
fn stripe_read_raw_panics_on_wrong_out_len() {
    let values = [Bits::<2>([1, 2]), Bits::<2>([3, 4])];
    let mut r = RAID1::<2, 2>([Bits::zero(); 2]);
    r.write_raw(&values);

    let mut out = [Bits::<2>::zero(); 1];
    r.read_raw(&mut out);
}

#[test]
fn stripe_as_restore_returns_some() {
    let r = RAID1::<2, 4>([Bits::zero(); 2]);
    assert!(r.as_restore().is_some());
}
