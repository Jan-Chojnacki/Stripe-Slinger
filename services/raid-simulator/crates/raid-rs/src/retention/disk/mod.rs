#[cfg(test)]
mod disk_tests;

use memmap2::{MmapMut, MmapOptions};
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Disk {
    path: PathBuf,
    file: Option<File>,
    map: Option<MmapMut>,
    len: u64,
    /// If true, the disk image exists but its contents are not trusted (e.g. newly created).
    pub needs_rebuild: bool,
}

impl Disk {
    /// # Errors
    /// Returns an error if the disk image cannot be created/opened or mapped.
    pub fn open_prealloc(path: &str, len: u64) -> anyhow::Result<Self> {
        let path = PathBuf::from(path);
        let existed = path.exists();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        let prev_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        file.set_len(len)?;

        let map_len = usize::try_from(len)
            .map_err(|_| anyhow::anyhow!("disk length {len} exceeds addressable size"))?;
        let map = unsafe { MmapOptions::new().len(map_len).map_mut(&file)? };

        Ok(Self {
            path,
            file: Some(file),
            map: Some(map),
            len,
            needs_rebuild: !existed || prev_len == 0,
        })
    }

    /// Mark this disk as failed (hot-remove).
    ///
    /// This will:
    /// - rename the underlying image to `*.failed.<ts>` (if it exists),
    /// - drop the mmap + file handle so the array stops using it.
    ///
    /// # Errors
    /// Returns an error if the disk image cannot be manipulated.
    pub fn fail(&mut self) -> anyhow::Result<()> {
        // Rename first so it's visible on the host filesystem even while the file is open.
        if self.path.exists() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let failed_path = self.path.with_extension(format!("img.failed.{ts}"));
            let _ = std::fs::rename(&self.path, &failed_path);
        }

        self.map.take();
        self.file.take();
        Ok(())
    }

    /// Replace this disk with a fresh, empty image (hot-swap). Contents must be rebuilt by RAID.
    ///
    /// # Errors
    /// Returns an error if the disk image cannot be recreated or mapped.
    pub fn replace(&mut self) -> anyhow::Result<()> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        file.set_len(self.len)?;
        let map_len = usize::try_from(self.len)
            .map_err(|_| anyhow::anyhow!("disk length {} exceeds addressable size", self.len))?;
        let map = unsafe { MmapOptions::new().len(map_len).map_mut(&file)? };

        self.file = Some(file);
        self.map = Some(map);
        self.needs_rebuild = true;
        Ok(())
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn len(&self) -> u64 {
        self.len
    }

    #[must_use]
    pub const fn is_operational(&self) -> bool {
        self.file.is_some() && self.map.is_some()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Missing from the array's point of view (failed / removed / unlinked).
    #[must_use]
    pub fn is_missing(&self) -> bool {
        if !self.is_operational() {
            return true;
        }
        self.file
            .as_ref()
            .and_then(|f| f.metadata().ok().map(|meta| meta.nlink() == 0))
            .unwrap_or(true)
    }

    pub fn read_at(&self, off: u64, buf: &mut [u8]) -> usize {
        let Some(map) = self.map.as_ref() else {
            return 0;
        };
        let Ok(off) = usize::try_from(off) else {
            return 0;
        };
        let Ok(disk_len) = usize::try_from(self.len) else {
            return 0;
        };
        if off >= disk_len {
            return 0;
        }
        let end = off.saturating_add(buf.len()).min(disk_len);
        let src = &map[off..end];
        let n = src.len();
        buf[..n].copy_from_slice(src);
        n
    }

    pub fn write_at(&mut self, off: u64, data: &[u8]) -> usize {
        let Some(map) = self.map.as_mut() else {
            return 0;
        };
        let Ok(off) = usize::try_from(off) else {
            return 0;
        };
        let Ok(disk_len) = usize::try_from(self.len) else {
            return 0;
        };
        if off >= disk_len {
            return 0;
        }
        let end = off.saturating_add(data.len()).min(disk_len);
        let dst = &mut map[off..end];
        let n = dst.len();
        dst.copy_from_slice(&data[..n]);
        // IMPORTANT:
        // Flushing every tiny write (our default chunk size is 4 bytes) makes startup rebuild and
        // read-repair extremely slow and can delay the FUSE mount from appearing.
        // This is a simulator; relying on the OS page cache is enough for visibility in hexdump.
        // If you need durability guarantees, add an explicit "sync" command and flush in batches.
        n
    }
}
