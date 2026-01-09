use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use fuser::MountOption;
use raid_rs::layout::stripe::raid0::RAID0;
use raid_rs::layout::stripe::raid1::RAID1;
use raid_rs::layout::stripe::raid3::RAID3;
use raid_rs::layout::stripe::traits::stripe::Stripe;
use raid_rs::retention::array::Array;
use raid_rs::retention::volume::Volume;

use crate::cli::RaidMode;
use crate::fs::{ENTRY_SIZE, Entry, FsState, HEADER_SIZE, Header, MAX_FILES, RaidFs};
use crate::metrics_runtime::MetricsEmitter;

fn disk_paths<const D: usize>(disk_dir: &Path) -> Result<[String; D]> {
    std::fs::create_dir_all(disk_dir)
        .with_context(|| format!("failed to create disk directory {}", disk_dir.display()))?;
    Ok(std::array::from_fn(|i| {
        disk_dir
            .join(format!("disk-{i}.img"))
            .to_string_lossy()
            .into_owned()
    }))
}

fn mount_volume<const D: usize, const N: usize, T>(
    mount_point: &Path,
    disk_dir: &Path,
    disk_size: u64,
    layout: T,
    metrics: std::sync::Arc<MetricsEmitter>,
) -> Result<()>
where
    T: Stripe<D, N> + Send + 'static,
{
    std::fs::create_dir_all(mount_point)
        .with_context(|| format!("failed to create mount point {}", mount_point.display()))?;
    let paths = disk_paths::<D>(disk_dir)?;
    let array = Array::<D, N>::init_array(&paths, disk_size);
    let capacity = array.disk_len().saturating_mul(T::DATA as u64);
    if capacity < RaidFs::<D, N, T>::data_start() + 1 {
        return Err(anyhow::anyhow!(
            "disk size too small for filesystem metadata"
        ));
    }
    let mut volume = Volume::new(array, layout);
    let mut header_buf = [0u8; HEADER_SIZE];
    volume.read_bytes(0, &mut header_buf);
    let parsed_header = RaidFs::<D, N, T>::parse_header(&header_buf);
    let is_new_header = parsed_header.is_none();
    let mut header = parsed_header.unwrap_or_else(|| Header {
        next_free: RaidFs::<D, N, T>::data_start(),
    });
    if header.next_free < RaidFs::<D, N, T>::data_start() {
        header.next_free = RaidFs::<D, N, T>::data_start();
    }

    let mut entries = vec![Entry::empty(); MAX_FILES];
    for (i, entry) in entries.iter_mut().enumerate().take(MAX_FILES) {
        let mut buf = [0u8; ENTRY_SIZE];
        let entry_offset = HEADER_SIZE as u64 + (i as u64 * ENTRY_SIZE as u64);
        volume.read_bytes(entry_offset, &mut buf);
        *entry = Entry::from_bytes(&buf);
    }

    if is_new_header {
        let header_bytes = RaidFs::<D, N, T>::header_bytes(&header);
        volume.write_bytes(0, &header_bytes);
        for (i, entry) in entries.iter_mut().enumerate().take(MAX_FILES) {
            let entry_offset = HEADER_SIZE as u64 + (i as u64 * ENTRY_SIZE as u64);
            let empty = Entry::empty().to_bytes();
            volume.write_bytes(entry_offset, &empty);
            *entry = Entry::empty();
        }

        volume.clear_needs_rebuild_all();
    }

    let state = Arc::new(Mutex::new(FsState {
        volume,
        header,
        entries,
    }));

    {
        let state_clone = state.clone();

        let rebuild_end = state_clone.lock().map_or_else(
            |_| RaidFs::<D, N, T>::data_start(),
            |st| st.header.next_free.max(RaidFs::<D, N, T>::data_start()),
        );

        let metrics_clone = metrics.clone();
        std::thread::spawn(move || {
            let stripes = {
                let Ok(st) = state_clone.lock() else {
                    return;
                };
                if st.volume.logical_capacity_bytes() == 0 {
                    return;
                }
                if st.volume.any_needs_rebuild() {
                    st.volume.stripes_needed_for_logical_end(rebuild_end)
                } else {
                    0
                }
            };

            if stripes == 0 {
                if let Ok(st) = state_clone.lock() {
                    record_status_snapshot(&metrics_clone, &st);
                }
                return;
            }

            let mut last_reported = 0;
            let report_every = (stripes / 100).max(1);

            for s in 0..stripes {
                if let Ok(mut st) = state_clone.lock() {
                    st.volume.repair_stripe(s);
                    if s + 1 >= last_reported + report_every || s + 1 == stripes {
                        let progress = (s + 1) as f64 / stripes as f64;
                        metrics_clone.record_raid_state(st.volume.failed_disks(), true, progress);
                        for status in st.volume.disk_statuses() {
                            metrics_clone.record_disk_status(status);
                        }
                        last_reported = s + 1;
                    }
                } else {
                    break;
                }
            }

            if let Ok(mut st) = state_clone.lock() {
                st.volume.clear_needs_rebuild_all();
                metrics_clone.record_raid_state(st.volume.failed_disks(), false, 1.0);
                for status in st.volume.disk_statuses() {
                    metrics_clone.record_disk_status(status);
                }
            }
        });
    }

    let fs = RaidFs {
        state,
        capacity,
        metrics: Some(metrics),
    };
    let mut options = vec![MountOption::RW, MountOption::FSName("raid-fuse".into())];
    if allow_other_enabled() {
        options.push(MountOption::AllowOther);
    }
    fuser::mount2(fs, mount_point, &options)
        .with_context(|| format!("failed to mount filesystem at {}", mount_point.display()))
}

fn record_status_snapshot<const D: usize, const N: usize, T>(
    metrics: &MetricsEmitter,
    state: &FsState<D, N, T>,
) where
    T: Stripe<D, N>,
{
    for status in state.volume.disk_statuses() {
        metrics.record_disk_status(status);
    }
    metrics.record_raid_state(
        state.volume.failed_disks(),
        state.volume.any_needs_rebuild(),
        0.0,
    );
}

fn allow_other_enabled() -> bool {
    let Ok(conf) = std::fs::read_to_string("/etc/fuse.conf") else {
        return false;
    };
    conf.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .any(|line| line == "user_allow_other")
}

pub fn run_fuse<const D: usize, const N: usize>(
    mode: RaidMode,
    mount_point: &Path,
    disk_dir: &Path,
    disk_size: u64,
    metrics: std::sync::Arc<MetricsEmitter>,
) -> Result<()> {
    match mode {
        RaidMode::Raid0 => mount_volume::<D, N, RAID0<D, N>>(
            mount_point,
            disk_dir,
            disk_size,
            RAID0::<D, N>::zero(),
            metrics,
        ),
        RaidMode::Raid1 => mount_volume::<D, N, RAID1<D, N>>(
            mount_point,
            disk_dir,
            disk_size,
            RAID1::<D, N>::zero(),
            metrics,
        ),
        RaidMode::Raid3 => mount_volume::<D, N, RAID3<D, N>>(
            mount_point,
            disk_dir,
            disk_size,
            RAID3::<D, N>::zero(),
            metrics,
        ),
    }
}
