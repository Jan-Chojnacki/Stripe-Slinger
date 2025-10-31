use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;

#[derive(Default)]
struct DummyStripe<const D: usize, const N: usize>;

impl<const D: usize, const N: usize> Stripe<D, N> for DummyStripe<D, N> {
    const DATA: usize = 0;
    const DISKS: usize = 0;
    fn write(&mut self, _data: &[Bits<N>]) {}
    fn write_raw(&mut self, data: &[Bits<N>]) {}
    fn read(&self, _out: &mut [Bits<N>]) {}
    fn read_raw(&self, out: &mut [Bits<N>]) {}
}

#[test]
fn default_as_restore_is_none_for_concrete_type() {
    let s = DummyStripe::<3, 4>::default();
    assert!(s.as_restore().is_none());
}
