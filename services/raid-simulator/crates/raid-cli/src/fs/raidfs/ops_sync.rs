use fuser::{ReplyEmpty, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::CTL_INO;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_flush(
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        if ino == CTL_INO || Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_release(
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        if ino == CTL_INO || Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }

    pub(crate) fn op_fsync(
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        if ino == CTL_INO || Self::index_for_inode(ino).is_some() {
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
