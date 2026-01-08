use fuser::{ReplyEmpty, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::*;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_flush(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        // Some tools (including `cat`) report errors if flush/close fails.
        // Our control file is virtual, so flush should always succeed.
        if ino == CTL_INO {
            reply.ok();
        } else if Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }

    pub(crate) fn op_release(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        // Release/close should never error for the virtual control file.
        if ino == CTL_INO {
            reply.ok();
        } else if Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }

    pub(crate) fn op_fsync(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        // No-op for this in-memory/fs-on-raid model; keep tools happy.
        if ino == CTL_INO || Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
