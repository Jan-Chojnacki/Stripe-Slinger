use std::ffi::OsStr;

use fuser::{FileType, ReplyDirectory, ReplyEntry, Request};
use raid_rs::layout::stripe::traits::stripe::Stripe;

use crate::fs::constants::{CTL_INO, CTL_NAME, ROOT_ID, TTL};

use super::types::RaidFs;

enum LookupTarget {
    Control,
    Entry(usize),
}

impl<const D: usize, const N: usize, T: Stripe<D, N>> RaidFs<D, N, T> {
    pub(crate) fn op_lookup(
        &self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        match self.lookup_target(parent, name) {
            Ok(LookupTarget::Control) => reply.entry(&TTL, &self.ctl_attr(), 0),
            Ok(LookupTarget::Entry(index)) => {
                let Ok(state) = self.state.lock() else {
                    reply.error(libc::EIO);
                    return;
                };
                reply.entry(&TTL, &self.entry_attr(index, state.entries[index].size), 0);
            }
            Err(code) => reply.error(code),
        }
    }

    pub(crate) fn op_readdir(
        &self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        match self.list_dir_entries(ino) {
            Ok(entries) => {
                let offset = usize::try_from(offset).unwrap_or(0);
                for (i, (inode, kind, name)) in entries.into_iter().enumerate().skip(offset) {
                    let next_offset = i64::try_from(i + 1).unwrap_or(i64::MAX);
                    if reply.add(inode, next_offset, kind, name.as_str()) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(code) => reply.error(code),
        }
    }

    fn lookup_target(&self, parent: u64, name: &OsStr) -> Result<LookupTarget, i32> {
        if parent != ROOT_ID {
            return Err(libc::ENOENT);
        }

        if name == OsStr::new(CTL_NAME) {
            return Ok(LookupTarget::Control);
        }

        let Ok(state) = self.state.lock() else {
            return Err(libc::EIO);
        };

        if let Some((index, _)) = state
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.used && entry.name == name.to_string_lossy())
        {
            Ok(LookupTarget::Entry(index))
        } else {
            Err(libc::ENOENT)
        }
    }

    fn list_dir_entries(&self, ino: u64) -> Result<Vec<(u64, FileType, String)>, i32> {
        if ino != ROOT_ID {
            return Err(libc::ENOENT);
        }

        let Ok(state) = self.state.lock() else {
            return Err(libc::EIO);
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

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::test_utils::create_test_fs;

    #[test]
    fn lookup_target_finds_control_and_entries() {
        let fs = create_test_fs();
        {
            let mut state = fs.state.lock().expect("lock state");
            state.entries[0].used = true;
            state.entries[0].name = "file.txt".to_string();
        }

        assert!(matches!(
            fs.lookup_target(ROOT_ID, OsStr::new(CTL_NAME)),
            Ok(LookupTarget::Control)
        ));
        assert!(matches!(
            fs.lookup_target(ROOT_ID, OsStr::new("file.txt")),
            Ok(LookupTarget::Entry(0))
        ));
    }

    #[test]
    fn list_dir_entries_includes_ctl_and_files() {
        let fs = create_test_fs();
        {
            let mut state = fs.state.lock().expect("lock state");
            state.entries[1].used = true;
            state.entries[1].name = "data.bin".to_string();
        }

        let entries = fs.list_dir_entries(ROOT_ID).expect("entries");
        assert!(entries.iter().any(|entry| entry.2 == CTL_NAME));
        assert!(entries.iter().any(|entry| entry.2 == "data.bin"));
    }

    #[test]
    fn list_dir_entries_rejects_non_root() {
        let fs = create_test_fs();
        let err = fs.list_dir_entries(999).expect_err("expected error");
        assert_eq!(err, libc::ENOENT);
    }
}
