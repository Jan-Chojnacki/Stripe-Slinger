use crate::retention::disk::Disk;
use rand::RngCore;
use tempfile::NamedTempFile;

const DISK_LEN: u64 = 1 << 20;

fn tmp_path_str(tf: &NamedTempFile) -> String {
    tf.path().to_string_lossy().into_owned()
}

#[test]
fn open_prealloc_creates_and_sizes_file() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);

    let d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");
    assert_eq!(d.len, DISK_LEN, "disk length must match requested");
    let meta = d.file.metadata().expect("metadata");
    assert_eq!(meta.len(), DISK_LEN, "backing file must be pre-sized");

    drop(d);
}

#[test]
fn initial_reads_are_zero_filled() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);

    let d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    let mut buf = vec![0xAAu8; 4096];
    let n = d.read_at(0, &mut buf);
    assert_eq!(n, 4096);
    assert!(
        buf.iter().all(|&b| b == 0),
        "newly allocated space should read as zeros"
    );

    let mut buf2 = vec![0xAAu8; 1234];
    let n2 = d.read_at(555_000, &mut buf2);
    assert_eq!(n2, 1234);
    assert!(buf2.iter().all(|&b| b == 0));
}

#[test]
fn write_then_read_roundtrip_same_session() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);
    let mut d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    let off = 64 * 1024 + 123;
    let mut data = vec![0u8; 8192];
    rand::rng().fill_bytes(&mut data);

    let wn = d.write_at(off, &data);
    assert_eq!(wn, data.len(), "must write full buffer");

    let mut back = vec![0u8; data.len()];
    let rn = d.read_at(off, &mut back);
    assert_eq!(rn, data.len(), "must read full buffer");
    assert_eq!(back, data, "roundtrip must match");
}

#[test]
fn durability_reopen_and_read_back() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);

    {
        let mut d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");
        let off = DISK_LEN / 2 - 200;
        let payload = b"hello-from-mmap!";
        let wn = d.write_at(off, payload);
        assert_eq!(wn, payload.len());
    }

    {
        let d2 = Disk::open_prealloc(&path, DISK_LEN).expect("reopen");
        let off = DISK_LEN / 2 - 200;
        let mut buf = vec![0u8; 16];
        let rn = d2.read_at(off, &mut buf);
        assert_eq!(rn, 16);
        assert_eq!(&buf, b"hello-from-mmap!");
    }
}

#[test]
fn read_past_end_is_truncated() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);
    let d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    let mut buf = vec![0xCCu8; 4096];
    let off = DISK_LEN - 512;
    let n = d.read_at(off, &mut buf);
    assert_eq!(n, 512, "read must truncate at EOF");
    assert!(buf[..512].iter().all(|&b| b == 0));
    assert!(
        buf[512..].iter().all(|&b| b == 0xCC),
        "untouched tail must remain"
    );
}

#[test]
fn write_past_end_is_truncated() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);
    let mut d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    let off = DISK_LEN - 100;
    let data = vec![0x5Au8; 500];
    let n = d.write_at(off, &data);
    assert_eq!(n, 100, "only the in-range prefix should be written");

    let mut back = vec![0u8; 128];
    let rn = d.read_at(DISK_LEN - 128, &mut back);
    assert_eq!(rn, 128);
    assert!(
        back[28..].iter().all(|&b| b == 0x5A),
        "suffix overlapping write must be 0x5A"
    );
    assert!(
        back[..28].iter().all(|&b| b == 0),
        "earlier bytes must remain zero"
    );
}

#[test]
fn overlapping_writes_behave_as_expected() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);
    let mut d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    let base = 256 * 1024;
    d.write_at(base, b"AAAAAAAAAA");
    d.write_at(base + 5, b"BBBBB");

    let mut buf = vec![0u8; 10];
    d.read_at(base, &mut buf);
    assert_eq!(&buf, b"AAAAABBBBB");
}

#[test]
fn large_random_roundtrips() {
    let tf = NamedTempFile::new().expect("tmp file");
    let path = tmp_path_str(&tf);
    let mut d = Disk::open_prealloc(&path, DISK_LEN).expect("open_prealloc");

    for _ in 0..8 {
        let off = (rand::random::<u64>() % (DISK_LEN - 8192)).min(DISK_LEN - 8192);
        let mut data = vec![0u8; 8192];
        rand::rng().fill_bytes(&mut data);
        let wn = d.write_at(off, &data);
        assert_eq!(wn, data.len());

        let mut back = vec![0u8; 8192];
        let rn = d.read_at(off, &mut back);
        assert_eq!(rn, back.len());
        assert_eq!(back, data);
    }
}
