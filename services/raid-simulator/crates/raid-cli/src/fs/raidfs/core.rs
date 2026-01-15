use std::ffi::OsStr;
use std::time::SystemTime;

use fuser::{FileAttr, FileType};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{
    CTL_INO, CTL_SIZE, FILE_ID_BASE, HEADER_SIZE, MAGIC, MAX_FILES, ROOT_ID, TABLE_SIZE, VERSION,
};
use crate::fs::metadata::Header;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    fn file_attr(ino: u64, size: u64) -> FileAttr {
        FileAttr {
            ino,
            size,
            blocks: size.div_ceil(512),
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }

    #[must_use]
    pub fn ctl_attr(&self) -> FileAttr {
        Self::file_attr(CTL_INO, CTL_SIZE)
    }

    #[must_use]
    pub const fn data_start() -> u64 {
        TABLE_SIZE as u64
    }

    #[must_use]
    pub fn header_bytes(header: &Header) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..8].copy_from_slice(&MAGIC);
        buf[8] = VERSION;
        buf[16..24].copy_from_slice(&header.next_free.to_le_bytes());
        let max_files = u32::try_from(MAX_FILES).unwrap_or(u32::MAX);
        buf[24..28].copy_from_slice(&max_files.to_le_bytes());
        buf
    }

    #[must_use]
    pub fn parse_header(buf: &[u8]) -> Option<Header> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        if buf[0..8] != MAGIC {
            return None;
        }
        if buf[8] != VERSION {
            return None;
        }
        let max_files = u32::from_le_bytes(buf[24..28].try_into().ok()?) as usize;
        if max_files != MAX_FILES {
            return None;
        }
        let next_free = u64::from_le_bytes(buf[16..24].try_into().ok()?);
        Some(Header { next_free })
    }

    #[must_use]
    pub const fn inode_for(index: usize) -> u64 {
        FILE_ID_BASE + index as u64
    }

    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn index_for_inode(ino: u64) -> Option<usize> {
        if ino < FILE_ID_BASE {
            None
        } else {
            let Ok(idx) = usize::try_from(ino - FILE_ID_BASE) else {
                return None;
            };
            if idx < MAX_FILES { Some(idx) } else { None }
        }
    }

    #[must_use]
    pub fn is_valid_name(name: &OsStr) -> bool {
        if name.is_empty() || name == OsStr::new(".") || name == OsStr::new("..") {
            return false;
        }
        !name.to_string_lossy().contains('/')
    }

    #[must_use]
    pub fn entry_attr(&self, index: usize, size: u64) -> FileAttr {
        Self::file_attr(Self::inode_for(index), size)
    }

    #[must_use]
    pub fn root_attr(&self) -> FileAttr {
        FileAttr {
            ino: ROOT_ID,
            size: 0,
            blocks: 0,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::test_utils::TestStripe;

    type TestFs = RaidFs<1, { crate::fs::DEFAULT_CHUNK_SIZE }, TestStripe>;

    #[test]
    fn header_bytes_round_trip() {
        let header = Header { next_free: 123 };
        let bytes = TestFs::header_bytes(&header);
        let parsed = TestFs::parse_header(&bytes).expect("parse header");
        assert_eq!(parsed.next_free, 123);
    }

    #[test]
    fn header_parse_rejects_bad_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(b"BADMAGIC");
        assert!(TestFs::parse_header(&bytes).is_none());
    }

    #[test]
    fn header_parse_rejects_bad_version() {
        let mut bytes = TestFs::header_bytes(&Header { next_free: 0 });
        bytes[8] = VERSION.saturating_add(1);
        assert!(TestFs::parse_header(&bytes).is_none());
    }

    #[test]
    fn header_parse_rejects_bad_max_files() {
        let mut bytes = TestFs::header_bytes(&Header { next_free: 0 });
        bytes[24..28].copy_from_slice(&(MAX_FILES as u32 + 1).to_le_bytes());
        assert!(TestFs::parse_header(&bytes).is_none());
    }

    #[test]
    fn header_parse_rejects_short_buffer() {
        let bytes = [0u8; HEADER_SIZE - 1];
        assert!(TestFs::parse_header(&bytes).is_none());
    }

    #[test]
    fn inode_mapping_round_trips() {
        let ino = TestFs::inode_for(5);
        assert_eq!(TestFs::index_for_inode(ino), Some(5));
        assert_eq!(TestFs::index_for_inode(FILE_ID_BASE - 1), None);
    }

    #[test]
    fn valid_name_rejects_paths() {
        assert!(!TestFs::is_valid_name(OsStr::new("")));
        assert!(!TestFs::is_valid_name(OsStr::new(".")));
        assert!(!TestFs::is_valid_name(OsStr::new("..")));
        assert!(!TestFs::is_valid_name(OsStr::new("a/b")));
        assert!(TestFs::is_valid_name(OsStr::new("file.txt")));
    }
}
