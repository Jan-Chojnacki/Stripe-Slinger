//! Filesystem constants for RAID-backed metadata layout.

use std::time::Duration;

/// ROOT_ID is the inode ID for the filesystem root.
pub const ROOT_ID: u64 = 1;
/// FILE_ID_BASE is the starting inode ID for regular files.
pub const FILE_ID_BASE: u64 = 2;
/// DEFAULT_DISK_LEN is the default disk image length in bytes.
pub const DEFAULT_DISK_LEN: u64 = 1024;
/// DEFAULT_CHUNK_SIZE is the default stripe chunk size in bytes.
pub const DEFAULT_CHUNK_SIZE: usize = 4;
/// TTL controls kernel cache TTL for attribute entries.
pub const TTL: Duration = Duration::from_secs(1);
/// MAGIC identifies the filesystem format on disk.
pub const MAGIC: [u8; 8] = *b"RAIDFS1\0";
/// VERSION is the on-disk format version.
pub const VERSION: u8 = 1;
/// NAME_LEN is the maximum filename length.
pub const NAME_LEN: usize = 64;
/// MAX_FILES is the maximum number of entries in the table.
pub const MAX_FILES: usize = 128;
/// HEADER_SIZE is the byte size of the metadata header.
pub const HEADER_SIZE: usize = 32;
/// ENTRY_SIZE is the byte size of each file entry.
pub const ENTRY_SIZE: usize = 88;
/// TABLE_SIZE is the total size of the header and entry table.
pub const TABLE_SIZE: usize = HEADER_SIZE + (ENTRY_SIZE * MAX_FILES);
/// OPEN_DIRECT_IO toggles direct I/O for FUSE file handles.
pub const OPEN_DIRECT_IO: u32 = 1;
/// STATFS_BLOCK_SIZE is the block size reported by statfs.
pub const STATFS_BLOCK_SIZE: u32 = 512;

/// CTL_NAME is the control file name exposed in the root directory.
pub const CTL_NAME: &str = ".raidctl";
/// CTL_INO is the inode number for the control file.
pub const CTL_INO: u64 = FILE_ID_BASE + (MAX_FILES as u64) + 1;
/// CTL_SIZE is the fixed size of the control file in bytes.
pub const CTL_SIZE: u64 = 4096;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_size_is_consistent() {
        assert_eq!(TABLE_SIZE, HEADER_SIZE + (ENTRY_SIZE * MAX_FILES));
    }

    #[test]
    fn ctl_inode_is_after_file_range() {
        assert_eq!(CTL_INO, FILE_ID_BASE + (MAX_FILES as u64) + 1);
    }
}
