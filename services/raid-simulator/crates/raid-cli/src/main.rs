use anyhow::Result;
use clap::Parser;

mod cli;
mod fs;
mod mount;

use cli::{Cli, Command, RaidMode};
use fs::DEFAULT_CHUNK_SIZE;
use mount::run_fuse;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Fuse {
            mount_point,
            disk_dir,
            raid,
            disks,
            disk_size,
        } => {
            let disk_size = disk_size.max(1);
            match (raid, disks) {
                (RaidMode::Raid0, 1) => {
                    run_fuse::<1, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 1) => Err(anyhow::anyhow!("raid mode requires at least 2 disks")),
                (_, 2) => {
                    run_fuse::<2, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 3) => {
                    run_fuse::<3, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 4) => {
                    run_fuse::<4, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 5) => {
                    run_fuse::<5, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 6) => {
                    run_fuse::<6, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 7) => {
                    run_fuse::<7, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                (_, 8) => {
                    run_fuse::<8, DEFAULT_CHUNK_SIZE>(raid, &mount_point, &disk_dir, disk_size)
                }
                _ => Err(anyhow::anyhow!(
                    "unsupported disk count {disks}; supported range is 1-8"
                )),
            }?;
        }
    }

    Ok(())
}
