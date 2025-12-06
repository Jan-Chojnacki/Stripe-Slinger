use super::*;
use crate::layout::stripe::raid0::RAID0;
use tempfile::TempDir;

const TEST_DISKS: usize = 3;
const CHUNK_SIZE: usize = 4;

fn disk_paths<const D: usize>(dir: &TempDir) -> [String; D] {
    std::array::from_fn(|i| {
        dir.path()
            .join(format!("disk-{i}.img"))
            .to_string_lossy()
            .into_owned()
    })
}

fn make_volume(
    paths: &[String; TEST_DISKS],
) -> Volume<TEST_DISKS, CHUNK_SIZE, RAID0<TEST_DISKS, CHUNK_SIZE>> {
    Volume::new(
        Array::init_array(paths.clone()),
        RAID0::<TEST_DISKS, CHUNK_SIZE>::zero(),
    )
}

#[test]
fn write_and_read_across_multiple_stripes() {
    let dir = TempDir::new().unwrap();
    let paths = disk_paths::<TEST_DISKS>(&dir);

    let mut volume = make_volume(&paths);
    let payload: Vec<u8> = (0..40).map(|i| i as u8).collect();
    volume.write_bytes(0, &payload);

    let mut volume = make_volume(&paths);
    let mut out = vec![0u8; 40];
    volume.read_bytes(0, &mut out);

    let expected: Vec<u8> = (0..40).map(|i| i as u8).collect();
    assert_eq!(out, expected);
}

#[test]
fn partial_write_preserves_unrelated_bytes() {
    let dir = TempDir::new().unwrap();
    let paths = disk_paths::<TEST_DISKS>(&dir);

    let initial: Vec<u8> = (0..30).map(|i| (i + 1) as u8).collect();

    let mut volume = make_volume(&paths);
    volume.write_bytes(0, &initial);

    let patch_offset = 5u64;
    let patch: Vec<u8> = (0..20).map(|i| (i + 200) as u8).collect();

    let mut volume = make_volume(&paths);
    volume.write_bytes(patch_offset, &patch);

    let mut volume = make_volume(&paths);
    let mut out = vec![0u8; initial.len()];
    volume.read_bytes(0, &mut out);

    let mut expected = initial.clone();
    expected[patch_offset as usize..patch_offset as usize + patch.len()].copy_from_slice(&patch);
    assert_eq!(out, expected);
}
