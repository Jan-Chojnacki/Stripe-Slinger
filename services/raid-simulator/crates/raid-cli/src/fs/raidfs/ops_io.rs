use fuser::{ReplyData, ReplyOpen, ReplyWrite, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;
use raid_rs::retention::volume::Volume;
use std::time::Instant;

use crate::fs::constants::{CTL_INO, OPEN_DIRECT_IO};
use crate::fs::persist::save_header_and_entry;
use crate::metrics_runtime::{FuseOp, FuseOpType};

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_open(&self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let start = Instant::now();
        let mut error = false;
        if ino == CTL_INO {
            reply.opened(CTL_INO, OPEN_DIRECT_IO);
            self.record_fuse_op(FuseOpType::Open, 0, start, error);
            return;
        }
        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            error = true;
            self.record_fuse_op(FuseOpType::Open, 0, start, error);
            return;
        };
        let Ok(state) = self.state.lock() else {
            reply.error(libc::EIO);
            error = true;
            self.record_fuse_op(FuseOpType::Open, 0, start, error);
            return;
        };
        if state.entries.get(index).is_some_and(|entry| entry.used) {
            reply.opened(ino, OPEN_DIRECT_IO);
        } else {
            reply.error(libc::ENOENT);
            error = true;
        }
        self.record_fuse_op(FuseOpType::Open, 0, start, error);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_read(
        &self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let start = Instant::now();
        let mut error = false;
        let mut bytes_sent: u64 = 0;
        if ino == CTL_INO {
            let Ok(state) = self.state.lock() else {
                reply.error(libc::EIO);
                error = true;
                self.record_fuse_op(FuseOpType::Read, 0, start, error);
                return;
            };
            let mut txt = String::new();
            txt.push_str("raidctl commands:\n");
            txt.push_str("  <n>           - fail disk n (hot-remove)\n");
            txt.push_str("  swap <n>      - fail + replace + rebuild disk n\n");
            txt.push_str("  replace <n>   - replace + rebuild disk n\n");
            txt.push_str("  rebuild <n>   - rebuild disk n\n\n");
            txt.push_str("disk status:\n");
            txt.push_str(&state.volume.disk_status_string());

            let bytes = txt.as_bytes();
            let off = usize::try_from(offset.max(0)).unwrap_or(0);
            let end = (off + size as usize).min(bytes.len());
            if off >= bytes.len() {
                reply.data(&[]);
            } else {
                reply.data(&bytes[off..end]);
                bytes_sent = u64::try_from(end.saturating_sub(off)).unwrap_or(0);
            }
            self.record_fuse_op(FuseOpType::Read, bytes_sent, start, error);
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            error = true;
            self.record_fuse_op(FuseOpType::Read, 0, start, error);
            return;
        };

        let offset = u64::try_from(offset.max(0)).unwrap_or(0);
        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            error = true;
            self.record_fuse_op(FuseOpType::Read, 0, start, error);
            return;
        };
        let Some(entry) = state.entries.get(index).filter(|entry| entry.used) else {
            reply.error(libc::ENOENT);
            error = true;
            self.record_fuse_op(FuseOpType::Read, 0, start, error);
            return;
        };

        let (file_offset, file_size) = (entry.offset, entry.size);
        if offset >= file_size {
            reply.data(&[]);
            self.record_fuse_op(FuseOpType::Read, 0, start, error);
            return;
        }

        let available = file_size - offset;
        let to_read = usize::try_from(u64::from(size).min(available)).unwrap_or(0);
        let mut buf = vec![0u8; to_read];
        let abs_offset = file_offset + offset;
        state.volume.read_bytes(abs_offset, &mut buf);
        reply.data(&buf);
        bytes_sent = u64::try_from(buf.len()).unwrap_or(0);
        self.record_fuse_op(FuseOpType::Read, bytes_sent, start, error);
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn op_write(
        &self,
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
        let start = Instant::now();
        let mut error = false;
        let mut bytes_written: u64 = 0;
        if ino == CTL_INO {
            let cmd = std::str::from_utf8(data).unwrap_or("").trim();

            let Ok(mut state) = self.state.lock() else {
                reply.error(libc::EIO);
                error = true;
                self.record_fuse_op(FuseOpType::Write, 0, start, error);
                return;
            };

            let end = state.header.next_free.max(Self::data_start());

            if let Ok(i) = cmd.parse::<usize>() {
                if state.volume.fail_disk(i).is_err() {
                    reply.error(libc::EINVAL);
                    error = true;
                    self.record_fuse_op(FuseOpType::Write, 0, start, error);
                    return;
                }
                bytes_written = u64::try_from(Self::write_len(data.len())).unwrap_or(0);
                reply.written(Self::write_len(data.len()));
                self.record_fuse_op(FuseOpType::Write, bytes_written, start, error);
                self.record_disk_and_raid_states(&state.volume, 0.0);
                return;
            }

            if let Some(rest) = cmd.strip_prefix("swap") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    let _ = state.volume.fail_disk(i);
                    if state.volume.replace_disk(i).is_err() {
                        reply.error(libc::EINVAL);
                        error = true;
                        self.record_fuse_op(FuseOpType::Write, 0, start, error);
                        return;
                    }
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        error = true;
                        self.record_fuse_op(FuseOpType::Write, 0, start, error);
                        return;
                    }
                    bytes_written = u64::try_from(Self::write_len(data.len())).unwrap_or(0);
                    reply.written(Self::write_len(data.len()));
                    self.record_fuse_op(FuseOpType::Write, bytes_written, start, error);
                    self.record_disk_and_raid_states(&state.volume, 0.0);
                    return;
                }
            }

            if let Some(rest) = cmd.strip_prefix("replace") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    if state.volume.replace_disk(i).is_err() {
                        reply.error(libc::EINVAL);
                        error = true;
                        self.record_fuse_op(FuseOpType::Write, 0, start, error);
                        return;
                    }
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        error = true;
                        self.record_fuse_op(FuseOpType::Write, 0, start, error);
                        return;
                    }
                    bytes_written = u64::try_from(Self::write_len(data.len())).unwrap_or(0);
                    reply.written(Self::write_len(data.len()));
                    self.record_fuse_op(FuseOpType::Write, bytes_written, start, error);
                    self.record_disk_and_raid_states(&state.volume, 0.0);
                    return;
                }
            }

            if let Some(rest) = cmd.strip_prefix("rebuild") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        error = true;
                        self.record_fuse_op(FuseOpType::Write, 0, start, error);
                        return;
                    }
                    bytes_written = u64::try_from(Self::write_len(data.len())).unwrap_or(0);
                    reply.written(Self::write_len(data.len()));
                    self.record_fuse_op(FuseOpType::Write, bytes_written, start, error);
                    self.record_disk_and_raid_states(&state.volume, 0.0);
                    return;
                }
            }

            reply.error(libc::EINVAL);
            error = true;
            self.record_fuse_op(FuseOpType::Write, 0, start, error);
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            error = true;
            self.record_fuse_op(FuseOpType::Write, 0, start, error);
            return;
        };

        let offset = u64::try_from(offset.max(0)).unwrap_or(0);
        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            error = true;
            self.record_fuse_op(FuseOpType::Write, 0, start, error);
            return;
        };
        let header_next_free = state.header.next_free;
        let Some(entry) = state.entries.get(index).filter(|entry| entry.used) else {
            reply.error(libc::ENOENT);
            error = true;
            self.record_fuse_op(FuseOpType::Write, 0, start, error);
            return;
        };
        let entry_offset = entry.offset;
        let entry_size = entry.size;

        let end_offset = offset.saturating_add(data.len() as u64);
        let new_size = entry_size.max(end_offset);
        let allocated = entry_size.max(1);
        let is_last = entry_offset + allocated == header_next_free;
        let new_allocated = new_size.max(1);
        let new_end = entry_offset.saturating_add(new_allocated);

        if new_end > self.capacity || (!is_last && new_size > entry.size) {
            reply.error(libc::ENOSPC);
            error = true;
            self.record_fuse_op(FuseOpType::Write, 0, start, error);
            return;
        }

        if offset > entry_size {
            let gap = usize::try_from(offset - entry_size).unwrap_or(0);
            if gap > 0 {
                let zeros = vec![0u8; gap];
                let gap_offset = entry_offset + entry_size;
                state.volume.write_bytes(gap_offset, &zeros);
            }
        }

        let abs_offset = entry_offset + offset;
        state.volume.write_bytes(abs_offset, data);
        if let Some(entry) = state.entries.get_mut(index) {
            entry.size = new_size;
        }
        if is_last {
            state.header.next_free = new_end;
        }
        save_header_and_entry(&mut state, index);
        reply.written(Self::write_len(data.len()));
        bytes_written = u64::try_from(Self::write_len(data.len())).unwrap_or(0);
        self.record_fuse_op(FuseOpType::Write, bytes_written, start, error);
    }

    fn write_len(len: usize) -> u32 {
        u32::try_from(len).unwrap_or(u32::MAX)
    }

    fn record_fuse_op(&self, op: FuseOpType, bytes: u64, start: Instant, error: bool) {
        if let Some(metrics) = self.metrics.as_ref() {
            metrics.record_fuse_op(FuseOp {
                op,
                bytes,
                latency_seconds: start.elapsed().as_secs_f64(),
                error,
            });
        }
    }

    fn record_disk_and_raid_states(&self, volume: &Volume<D, N, T>, progress: f64) {
        let Some(metrics) = self.metrics.as_ref() else {
            return;
        };
        for status in volume.disk_statuses() {
            metrics.record_disk_status(status);
        }
        let failed_disks = volume.failed_disks();
        let rebuild_in_progress = volume.any_needs_rebuild();
        metrics.record_raid_state(failed_disks, rebuild_in_progress, progress);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DEFAULT_CHUNK_SIZE;
    use crate::fs::test_utils::TestStripe;

    type TestFs = RaidFs<1, { DEFAULT_CHUNK_SIZE }, TestStripe>;

    #[test]
    fn write_len_clamps_to_u32() {
        assert_eq!(TestFs::write_len(0), 0);
        assert_eq!(TestFs::write_len(1), 1);
        assert_eq!(TestFs::write_len(u32::MAX as usize), u32::MAX);
        assert_eq!(TestFs::write_len((u32::MAX as usize) + 10), u32::MAX);
    }
}
