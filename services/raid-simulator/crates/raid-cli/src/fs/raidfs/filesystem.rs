use std::ffi::OsStr;
use std::time::SystemTime;

use fuser::{
    Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> Filesystem for RaidFs<D, N, T> {
    fn lookup(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        self.op_lookup(req, parent, name, reply);
    }

    fn getattr(&mut self, req: &Request<'_>, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        self.op_getattr(req, ino, fh, reply);
    }

    fn access(&mut self, req: &Request<'_>, ino: u64, mask: i32, reply: ReplyEmpty) {
        self.op_access(req, ino, mask, reply);
    }

    fn getxattr(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        Self::op_getxattr(req, ino, name, size, reply);
    }

    #[allow(clippy::similar_names)]
    fn setattr(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<u64>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        self.op_setattr(
            req, ino, mode, uid, gid, size, atime, mtime, ctime, fh, crtime, chgtime, bkuptime,
            flags, reply,
        );
    }

    fn mknod(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        self.op_mknod(req, parent, name, mode, umask, rdev, reply);
    }

    fn unlink(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        self.op_unlink(req, parent, name, reply);
    }

    fn open(&mut self, req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        self.op_open(req, ino, flags, reply);
    }

    fn read(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        self.op_read(req, ino, fh, offset, size, flags, lock_owner, reply);
    }

    fn write(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        self.op_write(
            req,
            ino,
            fh,
            offset,
            data,
            write_flags,
            flags,
            lock_owner,
            reply,
        );
    }

    fn flush(&mut self, req: &Request<'_>, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        Self::op_flush(req, ino, fh, lock_owner, reply);
    }

    fn release(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        flags: i32,
        lock_owner: Option<u64>,
        flush: bool,
        reply: ReplyEmpty,
    ) {
        Self::op_release(req, ino, fh, flags, lock_owner, flush, reply);
    }

    fn fsync(&mut self, req: &Request<'_>, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        self.op_fsync(req, ino, fh, datasync, reply);
    }

    fn readdir(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: ReplyDirectory,
    ) {
        self.op_readdir(req, ino, fh, offset, reply);
    }

    fn create(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        self.op_create(req, parent, name, mode, umask, flags, reply);
    }

    fn statfs(&mut self, req: &Request<'_>, ino: u64, reply: ReplyStatfs) {
        self.op_statfs(req, ino, reply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::test_utils::TestStripe;

    #[test]
    fn raidfs_implements_filesystem() {
        fn assert_impl<T: Filesystem>() {}
        assert_impl::<RaidFs<1, { DEFAULT_CHUNK_SIZE }, TestStripe>>();
    }
}
