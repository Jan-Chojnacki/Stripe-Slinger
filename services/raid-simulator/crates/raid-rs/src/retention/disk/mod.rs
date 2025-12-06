#[cfg(test)]
mod disk_tests;

use memmap2::{MmapMut, MmapOptions};
use std::fs::File;

pub struct Disk {
    file: File,
    map: MmapMut,
    len: u64,
}

impl Disk {
    pub fn open_prealloc(path: &str, len: u64) -> anyhow::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.set_len(len)?;
        let map = unsafe { MmapOptions::new().len(len as usize).map_mut(&file)? };
        Ok(Self { file, map, len })
    }

    pub fn read_at(&self, off: u64, buf: &mut [u8]) -> usize {
        let end = (off as usize)
            .saturating_add(buf.len())
            .min(self.len as usize);
        let src = &self.map[off as usize..end];
        let n = src.len();
        buf[..n].copy_from_slice(src);
        n
    }

    pub fn write_at(&mut self, off: u64, data: &[u8]) -> usize {
        let end = (off as usize)
            .saturating_add(data.len())
            .min(self.len as usize);
        let dst = &mut self.map[off as usize..end];
        let n = dst.len();
        dst.copy_from_slice(&data[..n]);
        let _ = self.map.flush_range(off as usize, n);
        n
    }
}
