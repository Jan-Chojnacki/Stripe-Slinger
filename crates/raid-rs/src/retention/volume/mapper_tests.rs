use super::*;

struct DummyStripe;

impl Stripe<3, 8> for DummyStripe {
    const DATA: usize = 2;
    const DISKS: usize = 3;

    fn write(&mut self, _: &[Bits<8>]) {}

    fn write_raw(&mut self, _: &[Bits<8>]) {}

    fn read(&self, _: &mut [Bits<8>]) {}

    fn read_raw(&self, _: &mut [Bits<8>]) {}
}

#[test]
fn geometry_uses_stripe_constants() {
    let geom = geometry::<3, 8, DummyStripe>();
    assert_eq!(geom.bytes_per_chunk, 8);
    assert_eq!(geom.bytes_per_stripe, 16);
}

#[test]
fn locate_byte_maps_offset_into_stripe() {
    let geom = Geometry {
        bytes_per_chunk: 8,
        bytes_per_stripe: 24,
    };

    let (stripe, in_stripe) = locate_byte(5, 30, &geom);
    assert_eq!(stripe, 1);
    assert_eq!(in_stripe, 11);

    let (stripe, in_stripe) = locate_byte(0, 48, &geom);
    assert_eq!(stripe, 2);
    assert_eq!(in_stripe, 0);
}

#[test]
#[should_panic(expected = "byte offset overflow")]
fn locate_byte_panics_on_overflow() {
    let geom = Geometry {
        bytes_per_chunk: 1,
        bytes_per_stripe: 1,
    };

    let _ = locate_byte(u64::MAX, 1, &geom);
}

#[test]
fn stripe_byte_offset_scales_index() {
    assert_eq!(stripe_byte_offset::<512>(0), 0);
    assert_eq!(stripe_byte_offset::<512>(5), 2560);
}

#[test]
#[should_panic(expected = "stripe offset overflow")]
fn stripe_byte_offset_panics_on_overflow() {
    let _ = stripe_byte_offset::<4>(u64::MAX);
}
