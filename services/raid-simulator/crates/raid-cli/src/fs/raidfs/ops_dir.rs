use std::ffi::OsStr;

use fuser::{FileType, ReplyDirectory, ReplyEntry, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::*;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        if parent != ROOT_ID {
            reply.error(libc::ENOENT);
            return;
        }

        if name == OsStr::new(CTL_NAME) {
            reply.entry(&TTL, &self.ctl_attr(), 0);
            return;
        }

        let Ok(state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        if let Some((index, entry)) = state
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.used && entry.name == name.to_string_lossy())
        {
            reply.entry(&TTL, &self.entry_attr(index, entry.size), 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    pub(crate) fn op_readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != ROOT_ID {
            reply.error(libc::ENOENT);
            return;
        }

        let Ok(state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };

        let mut entries: Vec<(u64, FileType, String)> = Vec::new();
        entries.push((ROOT_ID, FileType::Directory, ".".to_string()));
        entries.push((ROOT_ID, FileType::Directory, "..".to_string()));
        entries.push((CTL_INO, FileType::RegularFile, CTL_NAME.to_string()));
        for (index, entry) in state.entries.iter().enumerate() {
            if entry.used {
                entries.push((
                    Self::inode_for(index),
                    FileType::RegularFile,
                    entry.name.clone(),
                ));
            }
        }

        for (i, (inode, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            let next_offset = (i + 1) as i64;
            if reply.add(inode, next_offset, kind, name.as_str()) {
                break;
            }
        }
        reply.ok();
    }
}
