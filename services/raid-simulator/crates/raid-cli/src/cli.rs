use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::fs::DEFAULT_DISK_LEN;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Fuse(FuseArgs),

    Metrics(MetricsArgs),
}

#[derive(Args)]
pub struct FuseArgs {
    #[arg(long)]
    pub mount_point: PathBuf,

    #[arg(long)]
    pub disk_dir: PathBuf,

    #[arg(long, value_enum, default_value_t = RaidMode::Raid0)]
    pub raid: RaidMode,

    #[arg(long, default_value_t = 3)]
    pub disks: usize,

    #[arg(long, default_value_t = DEFAULT_DISK_LEN)]
    pub disk_size: u64,

    #[command(flatten)]
    pub metrics: MetricsArgs,
}

#[derive(Args, Debug, Clone)]
pub struct MetricsArgs {
    #[arg(
        long,
        env = "METRICS_SOCKET_PATH",
        default_value = "/sockets/metrics-gateway.sock"
    )]
    pub socket_path: String,

    #[arg(long, env = "METRICS_SOURCE_ID", default_value = "raid-simulator")]
    pub source_id: String,

    #[arg(long, env = "METRICS_INTERVAL_MS", default_value_t = 1000)]
    pub interval_ms: u64,

    #[arg(long, env = "METRICS_OPS_PER_TICK", default_value_t = 200)]
    pub ops_per_tick: u32,

    #[arg(long, env = "METRICS_QUEUE_CAP", default_value_t = 2048)]
    pub queue_cap: usize,

    #[arg(long, env = "METRICS_CONN_BUFFER", default_value_t = 512)]
    pub conn_buffer: usize,

    #[arg(long, env = "METRICS_CONNECT_TIMEOUT_MS", default_value_t = 2000)]
    pub connect_timeout_ms: u64,

    #[arg(long, env = "METRICS_RPC_TIMEOUT_MS", default_value_t = 0)]
    pub rpc_timeout_ms: u64,

    #[arg(long, env = "METRICS_BACKOFF_INITIAL_MS", default_value_t = 250)]
    pub backoff_initial_ms: u64,

    #[arg(long, env = "METRICS_BACKOFF_MAX_MS", default_value_t = 10_000)]
    pub backoff_max_ms: u64,

    #[arg(long, env = "METRICS_JITTER_RATIO", default_value_t = 0.2)]
    pub jitter_ratio: f64,

    #[arg(long, env = "METRICS_SHUTDOWN_GRACE_MS", default_value_t = 1500)]
    pub shutdown_grace_ms: u64,

    #[arg(long, env = "METRICS_AUTH_TOKEN", default_value = "")]
    pub auth_token: String,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum RaidMode {
    Raid0,
    Raid1,
    Raid3,
}
