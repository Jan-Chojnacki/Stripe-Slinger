use crate::layout::stripe::traits::stripe::Stripe;
use crate::retention::volume::Volume;

const FS_BLOCK_SIZE: u64 = 4096;
const MAGIC: &[u8; 8] = b"RAIDFS01";
const VERSION: u32 = 1;
const INODE_SIZE: usize = 128;
const INODE_COUNT: u32 = 1024;
const DIRECT_PTRS: usize = 12;
const UNALLOCATED_BLOCK: u32 = u32::MAX;

/// On-disk format
///
/// Layout is block-based with FS_BLOCK_SIZE (4096) blocks.
///
/// Block 0: superblock
/// - magic: b"RAIDFS01"
/// - version: u32
/// - fs_block_size: u32
/// - inode_count: u32
/// - inode_table_start_block: u32
/// - inode_table_blocks: u32
/// - bitmap_start_block: u32
/// - bitmap_blocks: u32
/// - data_start_block: u32
/// - next_inode: u32
///
/// Block 1..: inode table (fixed-size INODE_SIZE records)
///
/// Next: bitmap for data blocks (1 bit per data block)
///
/// Remaining: data blocks (file contents and directory entries)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Clone)]
pub struct FsAttr {
    pub kind: NodeKind,
    pub size: u64,
    pub mode: u16,
    pub nlink: u32,
}

#[derive(Debug)]
pub enum FsError {
    AlreadyExists,
    Corrupt,
    InvalidInput,
    IsDir,
    NoSpace,
    NotDir,
    NotEmpty,
    NotFound,
}

pub type FsResult<T> = Result<T, FsError>;

#[derive(Debug, Clone, Copy)]
struct Superblock {
    version: u32,
    fs_block_size: u32,
    inode_count: u32,
    inode_table_start_block: u32,
    inode_table_blocks: u32,
    bitmap_start_block: u32,
    bitmap_blocks: u32,
    data_start_block: u32,
    next_inode: u32,
}

#[derive(Debug, Clone, Copy)]
struct Inode {
    kind: InodeKind,
    mode: u16,
    size: u64,
    nlink: u32,
    direct: [u32; DIRECT_PTRS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InodeKind {
    Unused,
    File,
    Dir,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u32,
    pub kind: NodeKind,
    pub name: String,
}

pub struct VolumeFs<const D: usize, const N: usize, T: Stripe<D, N>> {
    volume: Volume<D, N, T>,
    superblock: Superblock,
    total_fs_blocks: u32,
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> VolumeFs<D, N, T> {
    pub fn mount_or_format(mut volume: Volume<D, N, T>, disk_len: u64) -> anyhow::Result<Self> {
        let total_fs_blocks = Self::total_fs_blocks(disk_len)?;
        let mut buf = vec![0u8; FS_BLOCK_SIZE as usize];
        volume.read_bytes(0, &mut buf);
        let fs = if let Some(superblock) = Superblock::from_bytes(&buf) {
            Self {
                volume,
                superblock,
                total_fs_blocks,
            }
        } else {
            let superblock = Self::format_volume(&mut volume, total_fs_blocks)?;
            Self {
                volume,
                superblock,
                total_fs_blocks,
            }
        };
        fs.validate_layout()
            .map_err(|err| anyhow::anyhow!("filesystem validation failed: {:?}", err))?;
        Ok(fs)
    }

    pub fn lookup(&mut self, parent: u32, name: &str) -> FsResult<u32> {
        let parent_inode = self.load_inode(parent)?;
        if parent_inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        let entries = self.read_dir_entries(&parent_inode)?;
        for entry in entries {
            if entry.name == name {
                return Ok(entry.inode);
            }
        }
        Err(FsError::NotFound)
    }

    pub fn getattr(&mut self, ino: u32) -> FsResult<FsAttr> {
        let inode = self.load_inode(ino)?;
        Ok(FsAttr {
            kind: inode.kind.to_node_kind()?,
            size: inode.size,
            mode: inode.mode,
            nlink: inode.nlink,
        })
    }

    pub fn readdir(&mut self, ino: u32) -> FsResult<Vec<DirEntry>> {
        let inode = self.load_inode(ino)?;
        if inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        self.read_dir_entries(&inode)
    }

    pub fn mkdir(&mut self, parent: u32, name: &str) -> FsResult<u32> {
        self.create_entry(parent, name, InodeKind::Dir, 0o755)
    }

    pub fn create(&mut self, parent: u32, name: &str) -> FsResult<u32> {
        self.create_entry(parent, name, InodeKind::File, 0o644)
    }

    pub fn read(&mut self, ino: u32, offset: u64, size: u32) -> FsResult<Vec<u8>> {
        let inode = self.load_inode(ino)?;
        if inode.kind == InodeKind::Dir {
            return Err(FsError::IsDir);
        }
        let size = size as u64;
        if offset >= inode.size {
            return Ok(Vec::new());
        }
        let to_read = size.min(inode.size - offset);
        self.read_file_data(&inode, offset, to_read)
    }

    pub fn write(&mut self, ino: u32, offset: u64, data: &[u8]) -> FsResult<usize> {
        let mut inode = self.load_inode(ino)?;
        if inode.kind == InodeKind::Dir {
            return Err(FsError::IsDir);
        }
        let end = offset.saturating_add(data.len() as u64);
        self.ensure_capacity(&mut inode, end)?;
        self.write_file_data(&mut inode, offset, data)?;
        if end > inode.size {
            inode.size = end;
        }
        self.store_inode(ino, &inode)?;
        Ok(data.len())
    }

    pub fn truncate(&mut self, ino: u32, new_size: u64) -> FsResult<()> {
        let mut inode = self.load_inode(ino)?;
        if inode.kind == InodeKind::Dir {
            return Err(FsError::IsDir);
        }
        let max_size = (DIRECT_PTRS as u64) * FS_BLOCK_SIZE;
        if new_size > max_size {
            return Err(FsError::InvalidInput);
        }
        if new_size > inode.size {
            let old_size = inode.size;
            self.ensure_capacity(&mut inode, new_size)?;
            self.zero_extend(&mut inode, old_size, new_size)?;
        } else if new_size < inode.size {
            self.shrink_inode(&mut inode, new_size)?;
        }
        inode.size = new_size;
        self.store_inode(ino, &inode)?;
        Ok(())
    }

    pub fn unlink(&mut self, parent: u32, name: &str) -> FsResult<()> {
        let (ino, kind) = self.remove_dir_entry(parent, name)?;
        if kind == NodeKind::Dir {
            return Err(FsError::IsDir);
        }
        self.free_inode(ino)
    }

    pub fn rmdir(&mut self, parent: u32, name: &str) -> FsResult<()> {
        let (ino, kind) = self.remove_dir_entry(parent, name)?;
        if kind != NodeKind::Dir {
            return Err(FsError::NotDir);
        }
        let entries = self.readdir(ino)?;
        if !entries.is_empty() {
            return Err(FsError::NotEmpty);
        }
        self.free_inode(ino)
    }

    pub fn rename(
        &mut self,
        old_parent: u32,
        old_name: &str,
        new_parent: u32,
        new_name: &str,
    ) -> FsResult<()> {
        let (ino, kind) = self.find_dir_entry(old_parent, old_name)?;
        if self.lookup(new_parent, new_name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        self.remove_dir_entry(old_parent, old_name)?;
        let entry = DirEntry {
            inode: ino,
            kind,
            name: new_name.to_string(),
        };
        self.append_dir_entry(new_parent, &entry)?;
        Ok(())
    }

    fn total_fs_blocks(disk_len: u64) -> anyhow::Result<u32> {
        if disk_len < FS_BLOCK_SIZE {
            anyhow::bail!("disk length too small for filesystem");
        }
        let stripes = disk_len / (N as u64);
        let capacity = stripes * (T::DATA as u64) * (N as u64);
        let total_fs_blocks = capacity / FS_BLOCK_SIZE;
        if total_fs_blocks == 0 {
            anyhow::bail!("volume capacity too small for filesystem");
        }
        Ok(total_fs_blocks as u32)
    }

    fn validate_layout(&self) -> FsResult<()> {
        if self.superblock.fs_block_size as u64 != FS_BLOCK_SIZE {
            return Err(FsError::Corrupt);
        }
        if self.superblock.inode_table_start_block != 1 {
            return Err(FsError::Corrupt);
        }
        if self.superblock.data_start_block <= self.superblock.bitmap_start_block {
            return Err(FsError::Corrupt);
        }
        Ok(())
    }

    fn format_volume(
        volume: &mut Volume<D, N, T>,
        total_fs_blocks: u32,
    ) -> anyhow::Result<Superblock> {
        let inode_table_start_block = 1u32;
        let inode_table_blocks =
            div_ceil((INODE_COUNT as u64) * (INODE_SIZE as u64), FS_BLOCK_SIZE) as u32;
        let remaining = total_fs_blocks
            .checked_sub(1 + inode_table_blocks)
            .ok_or_else(|| anyhow::anyhow!("not enough space for filesystem"))?;

        let (bitmap_blocks, data_blocks) = compute_bitmap_blocks(remaining)?;
        let bitmap_start_block = inode_table_start_block + inode_table_blocks;
        let data_start_block = bitmap_start_block + bitmap_blocks;

        let superblock = Superblock {
            version: VERSION,
            fs_block_size: FS_BLOCK_SIZE as u32,
            inode_count: INODE_COUNT,
            inode_table_start_block,
            inode_table_blocks,
            bitmap_start_block,
            bitmap_blocks,
            data_start_block,
            next_inode: 2,
        };

        let mut sb_block = vec![0u8; FS_BLOCK_SIZE as usize];
        superblock.write_bytes(&mut sb_block);
        volume.write_bytes(0, &sb_block);

        let zero_block = vec![0u8; FS_BLOCK_SIZE as usize];
        for block in inode_table_start_block..(inode_table_start_block + inode_table_blocks) {
            volume.write_bytes(block_offset(block), &zero_block);
        }
        for block in bitmap_start_block..(bitmap_start_block + bitmap_blocks) {
            volume.write_bytes(block_offset(block), &zero_block);
        }
        for block in data_start_block..(data_start_block + data_blocks) {
            volume.write_bytes(block_offset(block), &zero_block);
        }

        let root_inode = Inode::new(InodeKind::Dir, 0o755);
        write_inode_raw(volume, &superblock, 1, &root_inode)
            .map_err(|err| anyhow::anyhow!("failed to write root inode: {:?}", err))?;
        Ok(superblock)
    }

    fn load_inode(&mut self, ino: u32) -> FsResult<Inode> {
        if ino == 0 || ino > self.superblock.inode_count {
            return Err(FsError::NotFound);
        }
        let mut buf = [0u8; INODE_SIZE];
        self.volume.read_bytes(self.inode_offset(ino), &mut buf);
        let inode = Inode::from_bytes(&buf)?;
        if inode.kind == InodeKind::Unused {
            return Err(FsError::NotFound);
        }
        Ok(inode)
    }

    fn store_inode(&mut self, ino: u32, inode: &Inode) -> FsResult<()> {
        if ino == 0 || ino > self.superblock.inode_count {
            return Err(FsError::InvalidInput);
        }
        let mut buf = [0u8; INODE_SIZE];
        inode.write_bytes(&mut buf);
        self.volume.write_bytes(self.inode_offset(ino), &buf);
        Ok(())
    }

    fn inode_offset(&self, ino: u32) -> u64 {
        let index = ino - 1;
        block_offset(self.superblock.inode_table_start_block) + (index as u64) * (INODE_SIZE as u64)
    }

    fn data_blocks_count(&self) -> u32 {
        self.total_fs_blocks
            .saturating_sub(self.superblock.data_start_block)
    }

    fn allocate_inode(&mut self, kind: InodeKind, mode: u16) -> FsResult<u32> {
        for i in 1..=self.superblock.inode_count {
            let mut buf = [0u8; INODE_SIZE];
            self.volume.read_bytes(self.inode_offset(i), &mut buf);
            let inode = Inode::from_bytes(&buf)?;
            if inode.kind == InodeKind::Unused {
                let new_inode = Inode::new(kind, mode);
                self.store_inode(i, &new_inode)?;
                if i >= self.superblock.next_inode {
                    self.superblock.next_inode = i + 1;
                    self.save_superblock();
                }
                return Ok(i);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_inode(&mut self, ino: u32) -> FsResult<()> {
        let mut inode = self.load_inode(ino)?;
        self.shrink_inode(&mut inode, 0)?;
        inode.kind = InodeKind::Unused;
        inode.size = 0;
        inode.nlink = 0;
        inode.direct = [UNALLOCATED_BLOCK; DIRECT_PTRS];
        self.store_inode(ino, &inode)
    }

    fn create_entry(
        &mut self,
        parent: u32,
        name: &str,
        kind: InodeKind,
        mode: u16,
    ) -> FsResult<u32> {
        let parent_inode = self.load_inode(parent)?;
        if parent_inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        if self.lookup(parent, name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        let ino = self.allocate_inode(kind, mode)?;
        let entry = DirEntry {
            inode: ino,
            kind: kind.to_node_kind()?,
            name: name.to_string(),
        };
        self.append_dir_entry(parent, &entry)?;
        Ok(ino)
    }

    fn read_dir_entries(&mut self, inode: &Inode) -> FsResult<Vec<DirEntry>> {
        let data = self.read_file_data(inode, 0, inode.size)?;
        let mut entries = Vec::new();
        let mut offset = 0usize;
        while offset < data.len() {
            if offset + 7 > data.len() {
                return Err(FsError::Corrupt);
            }
            let inode_num = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            );
            let kind = match data[offset + 4] {
                1 => NodeKind::File,
                2 => NodeKind::Dir,
                _ => return Err(FsError::Corrupt),
            };
            let name_len = u16::from_le_bytes(
                data[offset + 5..offset + 7]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            ) as usize;
            offset += 7;
            if offset + name_len > data.len() {
                return Err(FsError::Corrupt);
            }
            let name = std::str::from_utf8(&data[offset..offset + name_len])
                .map_err(|_| FsError::Corrupt)?
                .to_string();
            offset += name_len;
            if inode_num != 0 {
                entries.push(DirEntry {
                    inode: inode_num,
                    kind,
                    name,
                });
            }
        }
        Ok(entries)
    }

    fn find_dir_entry(&mut self, parent: u32, name: &str) -> FsResult<(u32, NodeKind)> {
        let inode = self.load_inode(parent)?;
        if inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        let data = self.read_file_data(&inode, 0, inode.size)?;
        let mut offset = 0usize;
        while offset < data.len() {
            if offset + 7 > data.len() {
                return Err(FsError::Corrupt);
            }
            let inode_num = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            );
            let kind = match data[offset + 4] {
                1 => NodeKind::File,
                2 => NodeKind::Dir,
                _ => return Err(FsError::Corrupt),
            };
            let name_len = u16::from_le_bytes(
                data[offset + 5..offset + 7]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            ) as usize;
            offset += 7;
            if offset + name_len > data.len() {
                return Err(FsError::Corrupt);
            }
            let entry_name = std::str::from_utf8(&data[offset..offset + name_len])
                .map_err(|_| FsError::Corrupt)?;
            offset += name_len;
            if inode_num != 0 && entry_name == name {
                return Ok((inode_num, kind));
            }
        }
        Err(FsError::NotFound)
    }

    fn append_dir_entry(&mut self, parent: u32, entry: &DirEntry) -> FsResult<()> {
        let mut inode = self.load_inode(parent)?;
        if inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        let encoded = encode_dir_entry(entry)?;
        let offset = inode.size;
        self.ensure_capacity(&mut inode, offset + encoded.len() as u64)?;
        self.write_file_data(&mut inode, offset, &encoded)?;
        inode.size += encoded.len() as u64;
        self.store_inode(parent, &inode)
    }

    fn remove_dir_entry(&mut self, parent: u32, name: &str) -> FsResult<(u32, NodeKind)> {
        let mut inode = self.load_inode(parent)?;
        if inode.kind != InodeKind::Dir {
            return Err(FsError::NotDir);
        }
        let data = self.read_file_data(&inode, 0, inode.size)?;
        let mut offset = 0usize;
        while offset < data.len() {
            if offset + 7 > data.len() {
                return Err(FsError::Corrupt);
            }
            let inode_num = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            );
            let kind = match data[offset + 4] {
                1 => NodeKind::File,
                2 => NodeKind::Dir,
                _ => return Err(FsError::Corrupt),
            };
            let name_len = u16::from_le_bytes(
                data[offset + 5..offset + 7]
                    .try_into()
                    .map_err(|_| FsError::Corrupt)?,
            ) as usize;
            let entry_start = offset;
            offset += 7;
            if offset + name_len > data.len() {
                return Err(FsError::Corrupt);
            }
            let entry_name = std::str::from_utf8(&data[offset..offset + name_len])
                .map_err(|_| FsError::Corrupt)?;
            offset += name_len;
            if inode_num != 0 && entry_name == name {
                let zero = 0u32.to_le_bytes();
                self.write_file_data(&mut inode, entry_start as u64, &zero)?;
                self.store_inode(parent, &inode)?;
                return Ok((inode_num, kind));
            }
        }
        Err(FsError::NotFound)
    }

    fn read_file_data(&mut self, inode: &Inode, offset: u64, size: u64) -> FsResult<Vec<u8>> {
        if offset.saturating_add(size) > inode.size {
            return Err(FsError::InvalidInput);
        }
        let mut out = vec![0u8; size as usize];
        let mut read = 0u64;
        while read < size {
            let file_offset = offset + read;
            let block_index = (file_offset / FS_BLOCK_SIZE) as usize;
            let block_offset_in = (file_offset % FS_BLOCK_SIZE) as usize;
            let remaining_in_block = (FS_BLOCK_SIZE as usize) - block_offset_in;
            let take = remaining_in_block.min((size - read) as usize);
            let mut block_buf = vec![0u8; FS_BLOCK_SIZE as usize];
            if let Some(block) = inode.direct.get(block_index) {
                if *block != UNALLOCATED_BLOCK {
                    let abs_block = self.data_block_abs(*block)?;
                    self.volume
                        .read_bytes(block_offset(abs_block), &mut block_buf);
                }
            }
            out[(read as usize)..(read as usize + take)]
                .copy_from_slice(&block_buf[block_offset_in..block_offset_in + take]);
            read += take as u64;
        }
        Ok(out)
    }

    fn write_file_data(&mut self, inode: &mut Inode, offset: u64, data: &[u8]) -> FsResult<()> {
        let mut written = 0u64;
        let data_len = data.len() as u64;
        while written < data_len {
            let file_offset = offset + written;
            let block_index = (file_offset / FS_BLOCK_SIZE) as usize;
            let block_offset_in = (file_offset % FS_BLOCK_SIZE) as usize;
            let remaining_in_block = (FS_BLOCK_SIZE as usize) - block_offset_in;
            let take = remaining_in_block.min((data_len - written) as usize);
            let block_ptr = inode
                .direct
                .get_mut(block_index)
                .ok_or(FsError::InvalidInput)?;
            if *block_ptr == UNALLOCATED_BLOCK {
                *block_ptr = self.allocate_data_block()?;
            }
            let abs_block = self.data_block_abs(*block_ptr)?;
            let mut block_buf = vec![0u8; FS_BLOCK_SIZE as usize];
            if block_offset_in != 0 || take != FS_BLOCK_SIZE as usize {
                self.volume
                    .read_bytes(block_offset(abs_block), &mut block_buf);
            }
            block_buf[block_offset_in..block_offset_in + take]
                .copy_from_slice(&data[written as usize..written as usize + take]);
            self.volume.write_bytes(block_offset(abs_block), &block_buf);
            written += take as u64;
        }
        Ok(())
    }

    fn ensure_capacity(&mut self, inode: &mut Inode, end: u64) -> FsResult<()> {
        let max_size = (DIRECT_PTRS as u64) * FS_BLOCK_SIZE;
        if end > max_size {
            return Err(FsError::InvalidInput);
        }
        let needed_blocks = div_ceil(end, FS_BLOCK_SIZE) as usize;
        for idx in 0..needed_blocks {
            if inode.direct[idx] == UNALLOCATED_BLOCK {
                inode.direct[idx] = self.allocate_data_block()?;
            }
        }
        Ok(())
    }

    fn zero_extend(&mut self, inode: &mut Inode, from: u64, to: u64) -> FsResult<()> {
        if from >= to {
            return Ok(());
        }
        let mut offset = from;
        while offset < to {
            let block_index = (offset / FS_BLOCK_SIZE) as usize;
            let block_offset_in = (offset % FS_BLOCK_SIZE) as usize;
            let block_ptr = inode.direct[block_index];
            let abs_block = self.data_block_abs(block_ptr)?;
            let mut block_buf = vec![0u8; FS_BLOCK_SIZE as usize];
            self.volume
                .read_bytes(block_offset(abs_block), &mut block_buf);
            let block_end = (block_index as u64 + 1) * FS_BLOCK_SIZE;
            let end_in_block = (to.min(block_end) - block_index as u64 * FS_BLOCK_SIZE) as usize;
            for byte in &mut block_buf[block_offset_in..end_in_block] {
                *byte = 0;
            }
            self.volume.write_bytes(block_offset(abs_block), &block_buf);
            offset = block_end;
        }
        Ok(())
    }

    fn shrink_inode(&mut self, inode: &mut Inode, new_size: u64) -> FsResult<()> {
        let new_blocks = div_ceil(new_size, FS_BLOCK_SIZE) as usize;
        for idx in new_blocks..DIRECT_PTRS {
            if inode.direct[idx] != UNALLOCATED_BLOCK {
                self.free_data_block(inode.direct[idx])?;
                inode.direct[idx] = UNALLOCATED_BLOCK;
            }
        }
        Ok(())
    }

    fn allocate_data_block(&mut self) -> FsResult<u32> {
        let total = self.data_blocks_count();
        for idx in 0..total {
            if !self.is_block_allocated(idx)? {
                self.set_block_allocated(idx, true)?;
                let abs_block = self.data_block_abs(idx)?;
                let zero_block = vec![0u8; FS_BLOCK_SIZE as usize];
                self.volume
                    .write_bytes(block_offset(abs_block), &zero_block);
                return Ok(idx);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_data_block(&mut self, idx: u32) -> FsResult<()> {
        self.set_block_allocated(idx, false)
    }

    fn data_block_abs(&self, idx: u32) -> FsResult<u32> {
        let total = self.data_blocks_count();
        if idx >= total {
            return Err(FsError::InvalidInput);
        }
        Ok(self.superblock.data_start_block + idx)
    }

    fn is_block_allocated(&mut self, idx: u32) -> FsResult<bool> {
        let (byte_offset, bit) = self.bitmap_position(idx);
        let mut buf = [0u8; 1];
        self.volume.read_bytes(byte_offset, &mut buf);
        Ok((buf[0] & (1 << bit)) != 0)
    }

    fn set_block_allocated(&mut self, idx: u32, allocated: bool) -> FsResult<()> {
        let (byte_offset, bit) = self.bitmap_position(idx);
        let mut buf = [0u8; 1];
        self.volume.read_bytes(byte_offset, &mut buf);
        if allocated {
            buf[0] |= 1 << bit;
        } else {
            buf[0] &= !(1 << bit);
        }
        self.volume.write_bytes(byte_offset, &buf);
        Ok(())
    }

    fn bitmap_position(&self, idx: u32) -> (u64, u8) {
        let byte_index = idx / 8;
        let bit_in_byte = idx % 8;
        let offset = block_offset(self.superblock.bitmap_start_block) + byte_index as u64;
        (offset, bit_in_byte as u8)
    }

    fn save_superblock(&mut self) {
        let mut buf = vec![0u8; FS_BLOCK_SIZE as usize];
        self.superblock.write_bytes(&mut buf);
        self.volume.write_bytes(0, &buf);
    }

    #[cfg(test)]
    fn is_data_block_allocated(&mut self, idx: u32) -> bool {
        self.is_block_allocated(idx).unwrap_or(false)
    }
}

impl Superblock {
    fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 44 || &buf[..8] != MAGIC {
            return None;
        }
        let version = u32::from_le_bytes(buf[8..12].try_into().ok()?);
        if version != VERSION {
            return None;
        }
        Some(Self {
            version,
            fs_block_size: u32::from_le_bytes(buf[12..16].try_into().ok()?),
            inode_count: u32::from_le_bytes(buf[16..20].try_into().ok()?),
            inode_table_start_block: u32::from_le_bytes(buf[20..24].try_into().ok()?),
            inode_table_blocks: u32::from_le_bytes(buf[24..28].try_into().ok()?),
            bitmap_start_block: u32::from_le_bytes(buf[28..32].try_into().ok()?),
            bitmap_blocks: u32::from_le_bytes(buf[32..36].try_into().ok()?),
            data_start_block: u32::from_le_bytes(buf[36..40].try_into().ok()?),
            next_inode: u32::from_le_bytes(buf[40..44].try_into().ok()?),
        })
    }

    fn write_bytes(&self, buf: &mut [u8]) {
        buf[..8].copy_from_slice(MAGIC);
        buf[8..12].copy_from_slice(&self.version.to_le_bytes());
        buf[12..16].copy_from_slice(&self.fs_block_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.inode_count.to_le_bytes());
        buf[20..24].copy_from_slice(&self.inode_table_start_block.to_le_bytes());
        buf[24..28].copy_from_slice(&self.inode_table_blocks.to_le_bytes());
        buf[28..32].copy_from_slice(&self.bitmap_start_block.to_le_bytes());
        buf[32..36].copy_from_slice(&self.bitmap_blocks.to_le_bytes());
        buf[36..40].copy_from_slice(&self.data_start_block.to_le_bytes());
        buf[40..44].copy_from_slice(&self.next_inode.to_le_bytes());
    }
}

impl Inode {
    fn new(kind: InodeKind, mode: u16) -> Self {
        Self {
            kind,
            mode,
            size: 0,
            nlink: 1,
            direct: [UNALLOCATED_BLOCK; DIRECT_PTRS],
        }
    }

    fn from_bytes(buf: &[u8; INODE_SIZE]) -> FsResult<Self> {
        let kind = InodeKind::from_byte(buf[0])?;
        let mode = u16::from_le_bytes(buf[1..3].try_into().unwrap());
        let size = u64::from_le_bytes(buf[3..11].try_into().unwrap());
        let nlink = u32::from_le_bytes(buf[11..15].try_into().unwrap());
        let mut direct = [UNALLOCATED_BLOCK; DIRECT_PTRS];
        let mut offset = 15;
        for slot in &mut direct {
            let end = offset + 4;
            *slot = u32::from_le_bytes(buf[offset..end].try_into().unwrap());
            offset = end;
        }
        Ok(Self {
            kind,
            mode,
            size,
            nlink,
            direct,
        })
    }

    fn write_bytes(&self, buf: &mut [u8; INODE_SIZE]) {
        buf.fill(0);
        buf[0] = self.kind.to_byte();
        buf[1..3].copy_from_slice(&self.mode.to_le_bytes());
        buf[3..11].copy_from_slice(&self.size.to_le_bytes());
        buf[11..15].copy_from_slice(&self.nlink.to_le_bytes());
        let mut offset = 15;
        for slot in self.direct {
            let end = offset + 4;
            buf[offset..end].copy_from_slice(&slot.to_le_bytes());
            offset = end;
        }
    }
}

impl InodeKind {
    fn from_byte(byte: u8) -> FsResult<Self> {
        match byte {
            0 => Ok(Self::Unused),
            1 => Ok(Self::File),
            2 => Ok(Self::Dir),
            _ => Err(FsError::Corrupt),
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            Self::Unused => 0,
            Self::File => 1,
            Self::Dir => 2,
        }
    }

    fn to_node_kind(self) -> FsResult<NodeKind> {
        match self {
            Self::File => Ok(NodeKind::File),
            Self::Dir => Ok(NodeKind::Dir),
            Self::Unused => Err(FsError::NotFound),
        }
    }
}

fn write_inode_raw<const D: usize, const N: usize, T: Stripe<D, N>>(
    volume: &mut Volume<D, N, T>,
    superblock: &Superblock,
    ino: u32,
    inode: &Inode,
) -> FsResult<()> {
    let index = ino - 1;
    let offset =
        block_offset(superblock.inode_table_start_block) + (index as u64) * (INODE_SIZE as u64);
    let mut buf = [0u8; INODE_SIZE];
    inode.write_bytes(&mut buf);
    volume.write_bytes(offset, &buf);
    Ok(())
}

fn block_offset(block: u32) -> u64 {
    (block as u64) * FS_BLOCK_SIZE
}

fn div_ceil(a: u64, b: u64) -> u64 {
    if a == 0 { 0 } else { (a + b - 1) / b }
}

fn compute_bitmap_blocks(remaining: u32) -> anyhow::Result<(u32, u32)> {
    let mut bitmap_blocks = 1u32;
    loop {
        if remaining <= bitmap_blocks {
            anyhow::bail!("not enough space for data blocks");
        }
        let data_blocks = remaining - bitmap_blocks;
        let needed_bitmap = div_ceil(data_blocks as u64, FS_BLOCK_SIZE * 8) as u32;
        if needed_bitmap == bitmap_blocks {
            return Ok((bitmap_blocks, data_blocks));
        }
        bitmap_blocks = needed_bitmap;
    }
}

fn encode_dir_entry(entry: &DirEntry) -> FsResult<Vec<u8>> {
    let name_bytes = entry.name.as_bytes();
    if name_bytes.len() > u16::MAX as usize {
        return Err(FsError::InvalidInput);
    }
    let mut buf = Vec::with_capacity(7 + name_bytes.len());
    buf.extend_from_slice(&entry.inode.to_le_bytes());
    let kind = match entry.kind {
        NodeKind::File => 1u8,
        NodeKind::Dir => 2u8,
    };
    buf.push(kind);
    buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(name_bytes);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::stripe::raid3::RAID3;
    use crate::retention::array::Array;
    use tempfile::TempDir;

    fn build_fs() -> (TempDir, VolumeFs<3, 4, RAID3<3, 4>>) {
        let dir = TempDir::new().expect("tempdir");
        let disk_len = 16 * 1024 * 1024u64;
        let paths: [String; 3] = std::array::from_fn(|i| {
            dir.path()
                .join(format!("disk-{}.img", i))
                .to_string_lossy()
                .to_string()
        });
        let array = Array::<3, 4>::init_array_with_len(paths, disk_len).expect("array");
        let volume = Volume::<3, 4, RAID3<3, 4>>::new(array, RAID3::zero());
        let fs = VolumeFs::mount_or_format(volume, disk_len).expect("fs");
        (dir, fs)
    }

    #[test]
    fn formats_when_magic_missing() {
        let (_dir, mut fs) = build_fs();
        let root = fs.getattr(1).expect("root inode");
        assert_eq!(root.kind, NodeKind::Dir);
    }

    #[test]
    fn mkdir_create_write_read() {
        let (_dir, mut fs) = build_fs();
        let dir_ino = fs.mkdir(1, "docs").expect("mkdir");
        let file_ino = fs.create(dir_ino, "note.txt").expect("create");
        let payload = b"raid-fs";
        fs.write(file_ino, 0, payload).expect("write");
        let data = fs.read(file_ino, 0, payload.len() as u32).expect("read");
        assert_eq!(data, payload);
    }

    #[test]
    fn rename_moves_entry() {
        let (_dir, mut fs) = build_fs();
        fs.create(1, "a.txt").expect("create");
        fs.rename(1, "a.txt", 1, "b.txt").expect("rename");
        assert!(fs.lookup(1, "a.txt").is_err());
        assert!(fs.lookup(1, "b.txt").is_ok());
    }

    #[test]
    fn unlink_frees_blocks() {
        let (_dir, mut fs) = build_fs();
        let ino = fs.create(1, "big.bin").expect("create");
        let data = vec![0xAAu8; (FS_BLOCK_SIZE as usize) * 2];
        fs.write(ino, 0, &data).expect("write");
        assert!(fs.is_data_block_allocated(0));
        fs.unlink(1, "big.bin").expect("unlink");
        assert!(!fs.is_data_block_allocated(0));
    }

    #[test]
    fn truncate_grow_and_shrink() {
        let (_dir, mut fs) = build_fs();
        let ino = fs.create(1, "file.bin").expect("create");
        fs.truncate(ino, FS_BLOCK_SIZE * 2).expect("grow");
        let attr = fs.getattr(ino).expect("attr");
        assert_eq!(attr.size, FS_BLOCK_SIZE * 2);
        fs.truncate(ino, FS_BLOCK_SIZE / 2).expect("shrink");
        let attr = fs.getattr(ino).expect("attr");
        assert_eq!(attr.size, FS_BLOCK_SIZE / 2);
    }

    #[test]
    fn persistence_after_remount() {
        let (dir, mut fs) = build_fs();
        let ino = fs.create(1, "persist.txt").expect("create");
        fs.write(ino, 0, b"hello").expect("write");
        drop(fs);

        let disk_len = 16 * 1024 * 1024u64;
        let paths: [String; 3] = std::array::from_fn(|i| {
            dir.path()
                .join(format!("disk-{}.img", i))
                .to_string_lossy()
                .to_string()
        });
        let array = Array::<3, 4>::init_array_with_len(paths, disk_len).expect("array");
        let volume = Volume::<3, 4, RAID3<3, 4>>::new(array, RAID3::zero());
        let mut fs = VolumeFs::mount_or_format(volume, disk_len).expect("fs");
        let ino = fs.lookup(1, "persist.txt").expect("lookup");
        let data = fs.read(ino, 0, 5).expect("read");
        assert_eq!(data, b"hello");
    }
}
