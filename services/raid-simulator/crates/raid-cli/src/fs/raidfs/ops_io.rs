use fuser::{ReplyData, ReplyOpen, ReplyWrite, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{CTL_INO, OPEN_DIRECT_IO};
use crate::fs::persist::save_header_and_entry;

use super::types::RaidFs;

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_open(&self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        if ino == CTL_INO {
            reply.opened(CTL_INO, OPEN_DIRECT_IO);
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
        if state.entries.get(index).is_some_and(|entry| entry.used) {
            reply.opened(ino, OPEN_DIRECT_IO);
        } else {
            reply.error(libc::ENOENT);
        }
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
        if ino == CTL_INO {
            let Ok(state) = self.state.lock() else {
                reply.error(libc::EIO);
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
            }
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

        let offset = u64::try_from(offset.max(0)).unwrap_or(0);
        let Ok(mut state) = self.state.lock() else {
            reply.error(libc::EIO);
            return;
        };
        let Some(entry) = state.entries.get(index).filter(|entry| entry.used) else {
            reply.error(libc::ENOENT);
            return;
        };

        let (file_offset, file_size) = (entry.offset, entry.size);
        if offset >= file_size {
            reply.data(&[]);
            return;
        }

        let available = file_size - offset;
        let to_read = usize::try_from(u64::from(size).min(available)).unwrap_or(0);
        let mut buf = vec![0u8; to_read];
        let abs_offset = file_offset + offset;
        state.volume.read_bytes(abs_offset, &mut buf);
        reply.data(&buf);
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
        if ino == CTL_INO {
            let cmd = std::str::from_utf8(data).unwrap_or("").trim();

            let Ok(mut state) = self.state.lock() else {
                reply.error(libc::EIO);
                return;
            };

            let end = state.header.next_free.max(Self::data_start());

            // "<n>" => fail disk n (hot-remove)
            if let Ok(i) = cmd.parse::<usize>() {
                if state.volume.fail_disk(i).is_err() {
                    reply.error(libc::EINVAL);
                    return;
                }
                reply.written(Self::write_len(data.len()));
                return;
            }

            // "swap <n>" => fail + replace + rebuild
            if let Some(rest) = cmd.strip_prefix("swap") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    let _ = state.volume.fail_disk(i);
                    if state.volume.replace_disk(i).is_err() {
                        reply.error(libc::EINVAL);
                        return;
                    }
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        return;
                    }
                    reply.written(Self::write_len(data.len()));
                    return;
                }
            }

            // "replace <n>" => replace + rebuild
            if let Some(rest) = cmd.strip_prefix("replace") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    if state.volume.replace_disk(i).is_err() {
                        reply.error(libc::EINVAL);
                        return;
                    }
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        return;
                    }
                    reply.written(Self::write_len(data.len()));
                    return;
                }
            }

            // "rebuild <n>" => rebuild a disk that exists but is marked untrusted
            if let Some(rest) = cmd.strip_prefix("rebuild") {
                let rest = rest.trim();
                if let Ok(i) = rest.parse::<usize>() {
                    if state.volume.rebuild_disk_upto(i, end).is_err() {
                        reply.error(libc::EIO);
                        return;
                    }
                    reply.written(Self::write_len(data.len()));
                    return;
                }
            }

            reply.error(libc::EINVAL);
            return;
        }

        let Some(index) = Self::index_for_inode(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

        let offset = u64::try_from(offset.max(0)).unwrap_or(0);
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
        let entry_size = entry.size;

        let end_offset = offset.saturating_add(data.len() as u64);
        let new_size = entry_size.max(end_offset);
        let allocated = entry_size.max(1);
        let is_last = entry_offset + allocated == header_next_free;
        let new_allocated = new_size.max(1);
        let new_end = entry_offset.saturating_add(new_allocated);

        if new_end > self.capacity || (!is_last && new_size > entry.size) {
            reply.error(libc::ENOSPC);
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
    }

    fn write_len(len: usize) -> u32 {
        u32::try_from(len).unwrap_or(u32::MAX)
    }
}
