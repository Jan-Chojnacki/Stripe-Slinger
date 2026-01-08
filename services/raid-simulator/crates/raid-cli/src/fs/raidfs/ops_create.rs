use std::ffi::OsStr;

use fuser::{ReplyCreate, ReplyEmpty, ReplyEntry, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{CTL_INO, CTL_NAME, NAME_LEN, OPEN_DIRECT_IO, ROOT_ID, TTL};
use crate::fs::metadata::Entry;
use crate::fs::persist::save_header_and_entry;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_create(
        &self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        if parent != ROOT_ID || !Self::is_valid_name(name) {
            reply.error(libc::EINVAL);
            return;
        }

        if name == OsStr::new(CTL_NAME) {
            let attr = self.ctl_attr();
            reply.created(&TTL, &attr, 0, CTL_INO, OPEN_DIRECT_IO);
            return;
        }

        let name_str = name.to_string_lossy().into_owned();
        if name_str.len() > NAME_LEN {
            reply.error(libc::ENAMETOOLONG);
            return;
        }

        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        if state
            .entries
            .iter()
            .any(|entry| entry.used && entry.name == name_str)
        {
            reply.error(libc::EEXIST);
            return;
        }

        let Some(index) = state.entries.iter().position(|entry| !entry.used) else {
            reply.error(libc::ENOSPC);
            return;
        };

        let offset = state.header.next_free;
        let new_end = offset.saturating_add(1);
        if new_end > self.capacity {
            reply.error(libc::ENOSPC);
            return;
        }

        let entry = Entry {
            name: name_str,
            offset,
            size: 0,
            used: true,
        };
        state.entries[index] = entry;
        state.header.next_free = new_end;
        save_header_and_entry(&mut state, index);

        let attr = self.entry_attr(index, 0);
        reply.created(&TTL, &attr, 0, Self::inode_for(index), OPEN_DIRECT_IO);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_mknod(
        &self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        if parent != ROOT_ID || !Self::is_valid_name(name) {
            reply.error(libc::EINVAL);
            return;
        }

        let name_str = name.to_string_lossy().into_owned();
        if name_str.len() > NAME_LEN {
            reply.error(libc::ENAMETOOLONG);
            return;
        }

        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        if state
            .entries
            .iter()
            .any(|entry| entry.used && entry.name == name_str)
        {
            reply.error(libc::EEXIST);
            return;
        }

        let Some(index) = state.entries.iter().position(|entry| !entry.used) else {
            reply.error(libc::ENOSPC);
            return;
        };

        let offset = state.header.next_free;
        let new_end = offset.saturating_add(1);
        if new_end > self.capacity {
            reply.error(libc::ENOSPC);
            return;
        }

        let entry = Entry {
            name: name_str,
            offset,
            size: 0,
            used: true,
        };
        state.entries[index] = entry;
        state.header.next_free = new_end;
        save_header_and_entry(&mut state, index);

        let attr = self.entry_attr(index, 0);
        reply.entry(&TTL, &attr, 0);
    }

    pub(crate) fn op_unlink(
        &self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        if parent != ROOT_ID {
            reply.error(libc::ENOENT);
            return;
        }

        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        if let Some((index, _)) = state
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.used && entry.name == name.to_string_lossy())
        {
            state.entries[index] = Entry::empty();
            save_header_and_entry(&mut state, index);
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
