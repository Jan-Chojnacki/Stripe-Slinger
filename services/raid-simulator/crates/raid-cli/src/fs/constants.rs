use std::time::Duration;

pub const ROOT_ID: u64 = 1;
pub const FILE_ID_BASE: u64 = 2;
pub const DEFAULT_DISK_LEN: u64 = 1024;
pub const DEFAULT_CHUNK_SIZE: usize = 4;
pub const TTL: Duration = Duration::from_secs(1);
pub const MAGIC: [u8; 8] = *b"RAIDFS1\0";
pub const VERSION: u8 = 1;
pub const NAME_LEN: usize = 64;
pub const MAX_FILES: usize = 128;
pub const HEADER_SIZE: usize = 32;
pub const ENTRY_SIZE: usize = 88;
pub const TABLE_SIZE: usize = HEADER_SIZE + (ENTRY_SIZE * MAX_FILES);
pub const OPEN_DIRECT_IO: u32 = 1;
pub const STATFS_BLOCK_SIZE: u32 = 512;

pub const CTL_NAME: &str = ".raidctl";
pub const CTL_INO: u64 = FILE_ID_BASE + (MAX_FILES as u64) + 1;
pub const CTL_SIZE: u64 = 4096;
