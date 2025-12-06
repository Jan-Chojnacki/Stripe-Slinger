use crate::layout::bits::Bits;
use crate::layout::stripe::raid0::RAID0;
use crate::layout::stripe::traits::stripe::Stripe;

#[test]
fn zero_initializes_all_drives() {
    let r = RAID0::<3, 4>::zero();
    for drive in r.0.iter() {
        assert_eq!(drive.as_bytes(), &[0u8; 4]);
    }
}

#[test]
fn write_then_read_roundtrips_data() {
    let data = [
        Bits::<4>([1, 2, 3, 4]),
        Bits::<4>([5, 6, 7, 8]),
        Bits::<4>([9, 10, 11, 12]),
    ];
    let mut r = RAID0::<3, 4>::zero();

    r.write(&data);

    assert_eq!(r.0, data);

    let mut out = [Bits::<4>::zero(); <RAID0<3, 4> as Stripe<3, 4>>::DATA];
    r.read(&mut out);

    assert_eq!(out, data);
}
