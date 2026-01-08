use crate::layout::stripe::raid1::RAID1;

#[test]
fn zero_initializes_all_drives() {
    let r = RAID1::<3, 4>::zero();
    for d in 0..3 {
        assert_eq!(r.0[d].as_bytes(), &[0u8; 4], "drive {d}");
    }
}
