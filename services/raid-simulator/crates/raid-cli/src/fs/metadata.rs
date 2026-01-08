use super::constants::{ENTRY_SIZE, NAME_LEN};

#[derive(Clone, Debug)]
pub(crate) struct Header {
    pub(crate) next_free: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) used: bool,
}

impl Entry {
    pub(crate) fn empty() -> Self {
        Self {
            name: String::new(),
            offset: 0,
            size: 0,
            used: false,
        }
    }

    pub(crate) fn to_bytes(&self) -> [u8; ENTRY_SIZE] {
        let mut buf = [0u8; ENTRY_SIZE];
        buf[0] = if self.used { 1 } else { 0 };
        buf[8..16].copy_from_slice(&self.offset.to_le_bytes());
        buf[16..24].copy_from_slice(&self.size.to_le_bytes());
        let name_bytes = self.name.as_bytes();
        let max = name_bytes.len().min(NAME_LEN);
        buf[24..24 + max].copy_from_slice(&name_bytes[..max]);
        buf
    }

    pub(crate) fn from_bytes(buf: &[u8]) -> Self {
        let used = buf.get(0).copied().unwrap_or(0) == 1;
        let offset = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let size = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let name_bytes = &buf[24..24 + NAME_LEN];
        let end = name_bytes.iter().position(|b| *b == 0).unwrap_or(NAME_LEN);
        let name = String::from_utf8_lossy(&name_bytes[..end]).into_owned();
        Self {
            name,
            offset,
            size,
            used,
        }
    }
}
