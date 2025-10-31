use super::Array;
use crate::layout::bits::Bits;
use crate::layout::stripe::traits::stripe::Stripe;
use std::array::from_fn;
use tempfile::NamedTempFile;

fn tmp_paths<const D: usize>() -> ([NamedTempFile; D], [String; D]) {
    let temps: [NamedTempFile; D] = from_fn(|_| NamedTempFile::new().expect("tmp file"));
    let paths: [String; D] = from_fn(|i| temps[i].path().to_string_lossy().into_owned());
    (temps, paths)
}

struct SimpleStripe<const D: usize, const N: usize> {
    slots: [Bits<N>; D],
}

impl<const D: usize, const N: usize> SimpleStripe<D, N> {
    fn new(slots: [Bits<N>; D]) -> Self {
        Self { slots }
    }

    fn empty() -> Self {
        Self::new([Bits::zero(); D])
    }

    fn data(&self) -> [Bits<N>; D] {
        self.slots
    }
}

impl<const D: usize, const N: usize> Stripe<D, N> for SimpleStripe<D, N> {
    const DATA: usize = D;
    const DISKS: usize = D;

    fn write(&mut self, data: &[Bits<N>]) {
        self.write_raw(data);
    }

    fn write_raw(&mut self, data: &[Bits<N>]) {
        self.slots.copy_from_slice(&data[..D]);
    }

    fn read(&self, out: &mut [Bits<N>]) {
        self.read_raw(out);
    }

    fn read_raw(&self, out: &mut [Bits<N>]) {
        out[..D].copy_from_slice(&self.slots);
    }
}

#[test]
fn write_persists_data_to_each_disk() {
    const D: usize = 3;
    const N: usize = 16;
    let (_temps, paths) = tmp_paths::<D>();
    let mut array = Array::<D, N>::init_array(paths.clone());

    let write_data: [Bits<N>; D] = [
        Bits([0x11; N]),
        Bits([0x22; N]),
        Bits([0x33; N]),
    ];
    let stripe = SimpleStripe::new(write_data);
    let off = 256u64;

    array.write(off, &stripe);

    let expected = stripe.data();

    for disk in 0..D {
        let mut buf = [0u8; N];
        let n = array.0[disk].read_at(off, &mut buf);
        assert_eq!(n, N, "read back full stripe from disk");
        assert_eq!(buf, expected[disk].0, "disk contents must match stripe");
    }
}

#[test]
fn read_restores_data_into_stripe() {
    const D: usize = 4;
    const N: usize = 8;
    let (_temps, paths) = tmp_paths::<D>();
    let mut array = Array::<D, N>::init_array(paths.clone());

    let disk_contents: [Bits<N>; D] = [
        Bits([0xAA; N]),
        Bits([0xBB; N]),
        Bits([0xCC; N]),
        Bits([0xDD; N]),
    ];
    let off = 128u64;
    for disk in 0..D {
        let written = array.0[disk].write_at(off, &disk_contents[disk].0);
        assert_eq!(written, N, "all bytes must be written to disk");
    }

    let mut stripe = SimpleStripe::empty();
    array.read(off, &mut stripe);

    assert_eq!(stripe.data(), disk_contents, "stripe must match disk data");
}