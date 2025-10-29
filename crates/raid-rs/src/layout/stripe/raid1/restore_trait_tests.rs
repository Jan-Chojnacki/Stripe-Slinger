use crate::layout::bits::Bits;
use crate::layout::stripe::raid1::RAID1;
use crate::layout::stripe::traits::restore::Restore;

#[test]
fn restore_recovers_missing_drive_from_any_other() {
    let value = Bits::<4>([1, 2, 3, 4]);
    for missing in 0..3 {
        let mut r = RAID1::<3, 4>([value; 3]);
        r.0[missing] = Bits::zero();

        let restorer: &mut dyn Restore = &mut r;
        restorer.restore(missing);

        for drive in r.0.iter() {
            assert_eq!(*drive, value);
        }
    }
}

#[test]
#[should_panic]
fn restore_panics_on_invalid_index() {
    let value = Bits::<2>([0xAA, 0x55]);
    let mut r = RAID1::<2, 2>([value; 2]);

    let invalid = 2;
    let restorer: &mut dyn Restore = &mut r;
    restorer.restore(invalid);
}

#[test]
#[should_panic]
fn restore_panics_when_no_alternate_drive() {
    let value = Bits::<1>([1]);
    let mut r = RAID1::<1, 1>([value; 1]);

    let restorer: &mut dyn Restore = &mut r;
    restorer.restore(0);
}
