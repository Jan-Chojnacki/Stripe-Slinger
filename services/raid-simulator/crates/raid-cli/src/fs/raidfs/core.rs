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
