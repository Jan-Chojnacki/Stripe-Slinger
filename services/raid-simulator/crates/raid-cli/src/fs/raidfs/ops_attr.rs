use std::time::SystemTime;

use fuser::{ReplyAttr, ReplyEmpty, ReplyStatfs, ReplyXattr, Request, TimeOrNow};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{CTL_INO, MAX_FILES, NAME_LEN, ROOT_ID, STATFS_BLOCK_SIZE, TTL};
use crate::fs::persist::save_header_and_entry;

use super::types::RaidFs;

enum InodeTarget {
    Root,
    Control,
    Entry(usize),
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_access(&self, _req: &Request<'_>, ino: u64, _mask: i32, reply: ReplyEmpty) {
        match self.resolve_inode(ino) {
            Ok(_) => reply.ok(),
            Err(code) => reply.error(code),
        }
    }

    pub(crate) fn op_getxattr(
        &self,
        _req: &Request<'_>,
        ino: u64,
        _name: &std::ffi::OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        if !Self::is_inode_in_range(ino) {
            reply.error(libc::ENOENT);
            return;
        }

        if size == 0 {
            reply.size(0);
        } else {
            reply.data(&[]);
        }
    }

    pub(crate) fn op_getattr(
        &self,
        _req: &Request<'_>,
        ino: u64,
        _fh: Option<u64>,
        reply: ReplyAttr,
    ) {
        match self.resolve_inode(ino) {
            Ok(InodeTarget::Root) => reply.attr(&TTL, &self.root_attr()),
            Ok(InodeTarget::Control) => reply.attr(&TTL, &self.ctl_attr()),
            Ok(InodeTarget::Entry(index)) => {
                let Ok(state) = self.state.lock() else {
                    reply.error(libc::EIO);
                    return;
                };
                if let Some(entry) = state.entries.get(index).filter(|entry| entry.used) {
                    reply.attr(&TTL, &self.entry_attr(index, entry.size));
                } else {
                    reply.error(libc::ENOENT);
                }
            }
            Err(code) => reply.error(code),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_setattr(
        &self,
        _req: &Request<'_>,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        if ino == CTL_INO {
            reply.attr(&TTL, &self.ctl_attr());
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };
        let header_next_free = state.header.next_free;
        let Some(entry) = state.entries.get(index).filter(|entry| entry.used) else {
            reply.error(libc::ENOENT);
            return;
        };
        let entry_offset = entry.offset;
        let mut entry_size = entry.size;

        if let Some(new_size) = size {
            if new_size > entry_size {
                let allocated = entry_size.max(1);
                let is_last = entry_offset + allocated == header_next_free;
                let new_allocated = new_size.max(1);
                let new_end = entry_offset.saturating_add(new_allocated);
                if !is_last || new_end > self.capacity {
                    reply.error(libc::ENOSPC);
                    return;
                }
                state.header.next_free = new_end;
            }
            entry_size = new_size;
            if let Some(entry) = state.entries.get_mut(index) {
                entry.size = new_size;
            }
            save_header_and_entry(&mut state, index);
        }

        reply.attr(&TTL, &self.entry_attr(index, entry_size));
    }

    pub(crate) fn op_statfs(&self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        let Ok(state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        let used_bytes = state.header.next_free.max(Self::data_start());
        let available_bytes = self.capacity.saturating_sub(used_bytes);
        let block_size = u64::from(STATFS_BLOCK_SIZE);
        let blocks = self.capacity / block_size;
        let bfree = available_bytes / block_size;
        let bavail = bfree;
        let files = MAX_FILES as u64;
        let used_files = state.entries.iter().filter(|entry| entry.used).count() as u64;
        let ffree = files.saturating_sub(used_files);

        reply.statfs(
            blocks,
            bfree,
            bavail,
            files,
            ffree,
            STATFS_BLOCK_SIZE,
            NAME_LEN as u32,
            STATFS_BLOCK_SIZE,
        );
    }

    fn resolve_inode(&self, ino: u64) -> Result<InodeTarget, i32> {
        if ino == ROOT_ID {
            return Ok(InodeTarget::Root);
        }
        if ino == CTL_INO {
            return Ok(InodeTarget::Control);
        }

        let Some(index) = Self::index_for_inode(ino) else {
            return Err(libc::ENOENT);
        };

        let Ok(state) = self.state.lock() else {
            return Err(libc::EIO);
        };

        if state.entries.get(index).is_some_and(|entry| entry.used) {
            Ok(InodeTarget::Entry(index))
        } else {
            Err(libc::ENOENT)
        }
    }

    fn is_inode_in_range(ino: u64) -> bool {
        ino == ROOT_ID || ino == CTL_INO || Self::index_for_inode(ino).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::test_utils::TestStripe;
    use crate::fs::test_utils::create_test_fs;

    #[test]
    fn resolve_inode_recognizes_root_ctl_and_entries() {
        let fs = create_test_fs();
        {
            let mut state = fs.state.lock().expect("state lock");
            state.entries[0].used = true;
            state.entries[0].size = 1;
        }

        assert!(matches!(fs.resolve_inode(ROOT_ID), Ok(InodeTarget::Root)));
        assert!(matches!(
            fs.resolve_inode(CTL_INO),
            Ok(InodeTarget::Control)
        ));
        assert!(matches!(
            fs.resolve_inode(RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::inode_for(
                0
            )),
            Ok(InodeTarget::Entry(0))
        ));
        assert!(fs.resolve_inode(999_999).is_err());
    }

    #[test]
    fn resolve_inode_rejects_unused_entries() {
        let fs = create_test_fs();
        let ino = RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::inode_for(0);
        assert!(matches!(fs.resolve_inode(ino), Err(libc::ENOENT)));
    }

    #[test]
    fn inode_range_checks_root_ctl_and_entries() {
        assert!(RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::is_inode_in_range(ROOT_ID));
        assert!(RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::is_inode_in_range(CTL_INO));
        assert!(
            RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::is_inode_in_range(RaidFs::<
                1,
                { DEFAULT_CHUNK_SIZE },
                TestStripe,
            >::inode_for(
                0
            ))
        );
        assert!(!RaidFs::<1, { DEFAULT_CHUNK_SIZE }, TestStripe>::is_inode_in_range(999_999));
    }
}
