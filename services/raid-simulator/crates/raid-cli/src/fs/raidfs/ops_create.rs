use std::ffi::OsStr;

use fuser::{ReplyCreate, ReplyEmpty, ReplyEntry, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{CTL_INO, CTL_NAME, NAME_LEN, OPEN_DIRECT_IO, ROOT_ID, TTL};
use crate::fs::metadata::Entry;
use crate::fs::persist::save_header_and_entry;

use super::types::RaidFs;

enum CreateTarget {
    Control,
    Entry(usize),
}

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
        match self.create_target(parent, name) {
            Ok(CreateTarget::Control) => {
                let attr = self.ctl_attr();
                reply.created(&TTL, &attr, 0, CTL_INO, OPEN_DIRECT_IO);
            }
            Ok(CreateTarget::Entry(index)) => {
                let attr = self.entry_attr(index, 0);
                reply.created(&TTL, &attr, 0, Self::inode_for(index), OPEN_DIRECT_IO);
            }
            Err(code) => reply.error(code),
        }
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
        match self.create_regular_entry(parent, name) {
            Ok(index) => {
                let attr = self.entry_attr(index, 0);
                reply.entry(&TTL, &attr, 0);
            }
            Err(code) => reply.error(code),
        }
    }

    pub(crate) fn op_unlink(
        &self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        match self.unlink_entry(parent, name) {
            Ok(()) => reply.ok(),
            Err(code) => reply.error(code),
        }
    }

    fn create_target(&self, parent: u64, name: &OsStr) -> Result<CreateTarget, i32> {
        if parent != ROOT_ID || !Self::is_valid_name(name) {
            return Err(libc::EINVAL);
        }

        if name == OsStr::new(CTL_NAME) {
            return Ok(CreateTarget::Control);
        }

        let index = self.create_regular_entry(parent, name)?;
        Ok(CreateTarget::Entry(index))
    }

    fn create_regular_entry(&self, parent: u64, name: &OsStr) -> Result<usize, i32> {
        if parent != ROOT_ID || !Self::is_valid_name(name) {
            return Err(libc::EINVAL);
        }

        let name_str = name.to_string_lossy().into_owned();
        if name_str.len() > NAME_LEN {
            return Err(libc::ENAMETOOLONG);
        }

        let Ok(mut state) = self.state.lock() else {
            return Err(libc::EIO);
        };

        if state
            .entries
            .iter()
            .any(|entry| entry.used && entry.name == name_str)
        {
            return Err(libc::EEXIST);
        }

        let Some(index) = state.entries.iter().position(|entry| !entry.used) else {
            return Err(libc::ENOSPC);
        };

        let offset = state.header.next_free;
        let new_end = offset.saturating_add(1);
        if new_end > self.capacity {
            return Err(libc::ENOSPC);
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

        Ok(index)
    }

    fn unlink_entry(&self, parent: u64, name: &OsStr) -> Result<(), i32> {
        if parent != ROOT_ID {
            return Err(libc::ENOENT);
        }

        let Ok(mut state) = self.state.lock() else {
            return Err(libc::EIO);
        };

        if let Some((index, _)) = state
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.used && entry.name == name.to_string_lossy())
        {
            state.entries[index] = Entry::empty();
            save_header_and_entry(&mut state, index);
            Ok(())
        } else {
            Err(libc::ENOENT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::test_utils::create_test_fs;

    #[test]
    fn create_target_handles_control_name() {
        let fs = create_test_fs();
        let target = fs
            .create_target(ROOT_ID, OsStr::new(CTL_NAME))
            .expect("control target");
        assert!(matches!(target, CreateTarget::Control));
    }

    #[test]
    fn create_regular_entry_creates_entry() {
        let fs = create_test_fs();
        let index = fs
            .create_regular_entry(ROOT_ID, OsStr::new("file.txt"))
            .expect("create entry");
        let state = fs.state.lock().expect("lock state");
        assert!(state.entries[index].used);
        drop(state);
    }

    #[test]
    fn unlink_entry_removes_existing_entry() {
        let fs = create_test_fs();
        let index = fs
            .create_regular_entry(ROOT_ID, OsStr::new("deleteme"))
            .expect("create entry");
        assert!(fs.unlink_entry(ROOT_ID, OsStr::new("deleteme")).is_ok());
        let state = fs.state.lock().expect("lock state");
        assert!(!state.entries[index].used);
        drop(state);
    }

    #[test]
    fn create_regular_entry_rejects_invalid_parent() {
        let fs = create_test_fs();
        let err = fs
            .create_regular_entry(999, OsStr::new("file.txt"))
            .expect_err("expected error");
        assert_eq!(err, libc::EINVAL);
    }

    #[test]
    fn create_regular_entry_rejects_long_names() {
        let fs = create_test_fs();
        let long_name = "a".repeat(NAME_LEN + 1);
        let err = fs
            .create_regular_entry(ROOT_ID, OsStr::new(&long_name))
            .expect_err("expected error");
        assert_eq!(err, libc::ENAMETOOLONG);
    }

    #[test]
    fn create_regular_entry_rejects_duplicates() {
        let fs = create_test_fs();
        let _ = fs
            .create_regular_entry(ROOT_ID, OsStr::new("dupe"))
            .expect("create entry");
        let err = fs
            .create_regular_entry(ROOT_ID, OsStr::new("dupe"))
            .expect_err("expected error");
        assert_eq!(err, libc::EEXIST);
    }

    #[test]
    fn create_regular_entry_rejects_when_full() {
        let fs = create_test_fs();
        {
            let mut state = fs.state.lock().expect("lock state");
            for entry in &mut state.entries {
                entry.used = true;
            }
        }
        let err = fs
            .create_regular_entry(ROOT_ID, OsStr::new("full"))
            .expect_err("expected error");
        assert_eq!(err, libc::ENOSPC);
    }

    #[test]
    fn unlink_entry_returns_not_found() {
        let fs = create_test_fs();
        let err = fs
            .unlink_entry(ROOT_ID, OsStr::new("missing"))
            .expect_err("expected error");
        assert_eq!(err, libc::ENOENT);
    }
}
