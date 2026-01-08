use super::*;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[test]
fn writes_and_reads_file_with_checksum() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/data")).unwrap();

    let payload = b"hello, checksum";
    let meta = fs.write_file(Path::new("/data/file.txt"), payload).unwrap();

    assert_eq!(meta.size, payload.len() as u64);
    assert!(fs.verify(Path::new("/data/file.txt")).unwrap());

    let stored = fs.read_file(Path::new("/data/file.txt")).unwrap();
    assert_eq!(stored, payload);
}

#[test]
fn detects_corruption_on_verification() {
    let mut fs = ChecksumFs::new();
    fs.write_file(Path::new("file.bin"), b"abcdef").unwrap();

    if let Node::File(file) = fs.root.entries.get_mut("file.bin").unwrap() {
        file.data[0] = 0u8;
    }

    assert!(!fs.verify(Path::new("file.bin")).unwrap());
}

#[test]
fn lists_directory_entries_with_metadata() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/nested/dir")).unwrap();
    fs.write_file(Path::new("/nested/dir/note.md"), b"notes")
        .unwrap();
    fs.write_file(Path::new("/nested/dir/data.bin"), b"123")
        .unwrap();

    let entries = fs.list_dir(Path::new("/nested/dir")).unwrap();

    assert_eq!(entries.len(), 2);
    let mut names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["data.bin", "note.md"]);

    let note = entries.iter().find(|e| e.name == "note.md").unwrap();
    assert!(!note.is_dir);
    assert_eq!(note.metadata.as_ref().unwrap().size, 5);
}

#[test]
fn reports_metadata_for_root_and_files() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/logs")).unwrap();
    let payload = b"line";
    let meta = fs
        .write_file(Path::new("/logs/output.txt"), payload)
        .unwrap();

    let root_meta = fs.metadata(Path::new("")).unwrap();
    assert!(root_meta.is_dir);
    assert_eq!(root_meta.metadata, None);

    let file_meta = fs.metadata(Path::new("/logs/output.txt")).unwrap();
    assert!(!file_meta.is_dir);
    assert_eq!(file_meta.metadata.unwrap(), meta);
}

#[test]
fn reports_metadata_for_directories() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/logs/archive")).unwrap();

    let dir_meta = fs.metadata(Path::new("/logs"));

    let meta = dir_meta.expect("metadata should exist");
    assert_eq!(meta.name, "logs");
    assert!(meta.is_dir);
    assert!(meta.metadata.is_none());
}

#[test]
fn errors_when_paths_point_to_directories() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/configs/env"))
        .expect("mkdir_all should create nested directories");
    fs.write_file(Path::new("/configs/env/app"), b"APP=1")
        .expect("write_file should store file");

    let read_err = fs.read_file(Path::new("/configs/env"));
    assert!(read_err.unwrap_err().to_string().contains("is a directory"));

    let verify_err = fs.verify(Path::new("/configs/env"));
    assert!(
        verify_err
            .unwrap_err()
            .to_string()
            .contains("is a directory")
    );
}

#[test]
fn rejects_paths_with_parent_components() {
    let mut fs = ChecksumFs::new();
    let result = fs.write_file(Path::new("../escape"), b"nope");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("parent directory references are not supported")
    );
}

#[test]
fn errors_when_listing_missing_directory() {
    let fs = ChecksumFs::new();

    let err = fs.list_dir(Path::new("/missing"));

    assert!(
        err.unwrap_err()
            .to_string()
            .contains("missing directory missing")
    );
}

#[test]
fn mkdir_all_rejects_when_path_conflicts_with_file() {
    let mut fs = ChecksumFs::new();
    fs.write_file(Path::new("/conflict"), b"file").unwrap();

    let err = fs.mkdir_all(Path::new("/conflict/dir"));

    assert!(err.unwrap_err().to_string().contains("is a file"));
}

#[test]
fn errors_for_non_utf8_components() {
    let mut path = PathBuf::from("/data");
    path.push(OsStr::from_bytes(&[0xff, 0xfe]));

    let err = ChecksumFs::new().write_file(&path, b"bad");

    assert!(
        err.unwrap_err()
            .to_string()
            .contains("non utf-8 path component")
    );
}

#[test]
fn read_fails_when_file_missing() {
    let fs = ChecksumFs::new();

    let err = fs.read_file(Path::new("/nope.bin"));

    assert!(err.unwrap_err().to_string().contains("nope.bin not found"));
}

#[test]
fn write_rejects_empty_paths() {
    let err = ChecksumFs::new().write_file(Path::new(""), b"data");

    assert!(err.unwrap_err().to_string().contains("path is empty"));
}

#[test]
fn read_fails_when_parent_directory_missing() {
    let fs = ChecksumFs::new();

    let err = fs.read_file(Path::new("/missing/path.txt"));

    assert!(
        err.unwrap_err()
            .to_string()
            .contains("missing directory missing")
    );
}

#[test]
fn list_dir_fails_when_path_points_to_file() {
    let mut fs = ChecksumFs::new();
    fs.write_file(Path::new("/data.bin"), b"123").unwrap();

    let err = fs.list_dir(Path::new("/data.bin"));

    assert!(err.unwrap_err().to_string().contains("data.bin is a file"));
}

#[test]
fn lists_directory_entries_including_directories() {
    let mut fs = ChecksumFs::new();
    fs.mkdir_all(Path::new("/data/bin")).unwrap();
    fs.write_file(Path::new("/data/info.txt"), b"info").unwrap();

    let mut entries = fs.list_dir(Path::new("/data")).unwrap();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(entries.len(), 2);

    let dir_entry = entries.iter().find(|e| e.name == "bin").unwrap();
    assert!(dir_entry.is_dir);
    assert!(dir_entry.metadata.is_none());

    let file_entry = entries.iter().find(|e| e.name == "info.txt").unwrap();
    assert!(!file_entry.is_dir);
    assert_eq!(file_entry.metadata.as_ref().unwrap().size, 4);
}
