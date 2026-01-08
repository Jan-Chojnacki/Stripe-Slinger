use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::fs::DEFAULT_DISK_LEN;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    Fuse {
        #[arg(long)]
        mount_point: PathBuf,
        #[arg(long)]
        disk_dir: PathBuf,
        #[arg(long, value_enum, default_value_t = RaidMode::Raid0)]
        raid: RaidMode,
        #[arg(long, default_value_t = 3)]
        disks: usize,
        #[arg(long, default_value_t = DEFAULT_DISK_LEN)]
        disk_size: u64,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum RaidMode {
    Raid0,
    Raid1,
    Raid3,
}
