use std::time::SystemTime;

use fuser::{ReplyAttr, Request, TimeOrNow};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::*;
use crate::fs::persist::save_header_and_entry;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_getattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: Option<u64>,
        reply: ReplyAttr,
    ) {
        if ino == ROOT_ID {
            reply.attr(&TTL, &self.root_attr());
            return;
        }

        if ino == CTL_INO {
            reply.attr(&TTL, &self.ctl_attr());
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

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

    pub(crate) fn op_setattr(
        &mut self,
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
        // Allow shells to use redirections like `echo 2 > .raidctl`.
        // Such redirections typically open with O_TRUNC which triggers setattr(size=0)
        // *before* write(). We treat the control file as a virtual file: we accept
        // truncation/size changes without affecting anything and simply return its attrs.
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
            let _ = save_header_and_entry(&mut state, index);
        }

        reply.attr(&TTL, &self.entry_attr(index, entry_size));
    }
}
