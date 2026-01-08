use std::ffi::OsStr;
use std::time::SystemTime;

use fuser::{
    Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyWrite, Request, TimeOrNow,
};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> Filesystem for RaidFs<D, N, T> {
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        self.op_getattr(_req, ino, _fh, reply);
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        self.op_lookup(_req, parent, name, reply);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        reply: ReplyDirectory,
    ) {
        self.op_readdir(_req, ino, _fh, offset, reply);
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        self.op_open(_req, ino, _flags, reply);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        self.op_read(_req, ino, _fh, offset, size, _flags, _lock_owner, reply);
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        self.op_write(
            _req,
            ino,
            _fh,
            offset,
            data,
            _write_flags,
            _flags,
            _lock_owner,
            reply,
        );
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        self.op_create(_req, parent, name, _mode, _umask, _flags, reply);
    }

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        self.op_mknod(_req, parent, name, _mode, _umask, _rdev, reply);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        self.op_unlink(_req, parent, name, reply);
    }

    fn setattr(
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
        self.op_setattr(
            _req, ino, _mode, _uid, _gid, size, _atime, _mtime, _ctime, _fh, _crtime, _chgtime,
            _bkuptime, _flags, reply,
        );
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        self.op_flush(_req, ino, _fh, _lock_owner, reply);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        self.op_release(_req, ino, _fh, _flags, _lock_owner, _flush, reply);
    }

    fn fsync(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        self.op_fsync(_req, ino, _fh, _datasync, reply);
    }
}
