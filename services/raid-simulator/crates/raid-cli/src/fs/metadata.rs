use super::constants::{ENTRY_SIZE, NAME_LEN};

#[derive(Clone, Debug)]
pub struct Header {
    pub next_free: u64,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub used: bool,
}

#[allow(clippy::missing_const_for_fn)]
impl Entry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            offset: 0,
            size: 0,
            used: false,
        }
    }

    #[must_use]
    pub fn to_bytes(&self) -> [u8; ENTRY_SIZE] {
        let mut buf = [0u8; ENTRY_SIZE];
        buf[0] = u8::from(self.used);
        buf[8..16].copy_from_slice(&self.offset.to_le_bytes());
        buf[16..24].copy_from_slice(&self.size.to_le_bytes());
        let name_bytes = self.name.as_bytes();
        let max = name_bytes.len().min(NAME_LEN);
        buf[24..24 + max].copy_from_slice(&name_bytes[..max]);
        buf
    }

    /// # Panics
    /// Panics if `buf` does not contain a full entry payload.
    #[must_use]
    pub fn from_bytes(buf: &[u8]) -> Self {
        let used = buf.first().copied().unwrap_or(0) == 1;
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
