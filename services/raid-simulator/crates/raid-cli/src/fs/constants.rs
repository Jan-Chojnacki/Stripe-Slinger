use std::time::Duration;

pub(crate) const ROOT_ID: u64 = 1;
pub(crate) const FILE_ID_BASE: u64 = 2;
pub(crate) const DEFAULT_DISK_LEN: u64 = 1024;
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 4;
pub(crate) const TTL: Duration = Duration::from_secs(1);
pub(crate) const MAGIC: [u8; 8] = *b"RAIDFS1\0";
pub(crate) const VERSION: u8 = 1;
pub(crate) const NAME_LEN: usize = 64;
pub(crate) const MAX_FILES: usize = 128;
pub(crate) const HEADER_SIZE: usize = 32;
pub(crate) const ENTRY_SIZE: usize = 88;
pub(crate) const TABLE_SIZE: usize = HEADER_SIZE + (ENTRY_SIZE * MAX_FILES);
pub(crate) const OPEN_DIRECT_IO: u32 = 1;

pub(crate) const CTL_NAME: &str = ".raidctl";
pub(crate) const CTL_INO: u64 = FILE_ID_BASE + (MAX_FILES as u64) + 1;
pub(crate) const CTL_SIZE: u64 = 4096;
