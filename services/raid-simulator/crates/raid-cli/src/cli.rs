//! Command-line argument definitions for the RAID simulator CLI.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::fs::DEFAULT_DISK_LEN;

/// Cli defines the root command for the RAID simulator binary.
#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Command enumerates the supported CLI subcommands.
#[derive(Subcommand)]
pub enum Command {
    Fuse(FuseArgs),

    Metrics(MetricsArgs),
}

/// `FuseArgs` configures the FUSE mount command.
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

    #[arg(long, default_value_t = false)]
    pub allow_other: bool,
}

/// `MetricsArgs` configures metrics streaming options.
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

    #[arg(long, env = "GRPC_AUTH_TOKEN", default_value = "")]
    pub auth_token: String,
}

/// `RaidMode` selects the RAID layout for the simulation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RaidMode {
    Raid0,
    Raid1,
    Raid3,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn clear(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                unsafe {
                    std::env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn parses_fuse_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _socket = EnvGuard::clear("METRICS_SOCKET_PATH");
        let _source = EnvGuard::clear("METRICS_SOURCE_ID");
        let _interval = EnvGuard::clear("METRICS_INTERVAL_MS");
        let _ops = EnvGuard::clear("METRICS_OPS_PER_TICK");
        let _queue = EnvGuard::clear("METRICS_QUEUE_CAP");
        let _conn = EnvGuard::clear("METRICS_CONN_BUFFER");
        let _connect = EnvGuard::clear("METRICS_CONNECT_TIMEOUT_MS");
        let _rpc = EnvGuard::clear("METRICS_RPC_TIMEOUT_MS");
        let _backoff_initial = EnvGuard::clear("METRICS_BACKOFF_INITIAL_MS");
        let _backoff_max = EnvGuard::clear("METRICS_BACKOFF_MAX_MS");
        let _jitter = EnvGuard::clear("METRICS_JITTER_RATIO");
        let _shutdown = EnvGuard::clear("METRICS_SHUTDOWN_GRACE_MS");
        let _auth = EnvGuard::clear("GRPC_AUTH_TOKEN");

        let cli = Cli::parse_from([
            "raid-cli",
            "fuse",
            "--mount-point",
            "/mnt/raid",
            "--disk-dir",
            "/var/raid",
        ]);

        let Command::Fuse(args) = cli.command else {
            panic!("expected fuse command");
        };

        assert_eq!(args.raid, RaidMode::Raid0);
        assert_eq!(args.disks, 3);
        assert_eq!(args.disk_size, DEFAULT_DISK_LEN);
        assert_eq!(args.metrics.interval_ms, 1000);
        assert_eq!(args.metrics.ops_per_tick, 200);
        assert_eq!(args.metrics.queue_cap, 2048);
    }

    #[test]
    fn parses_metrics_with_env_overrides() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _socket = EnvGuard::set("METRICS_SOCKET_PATH", "/tmp/metrics.sock");
        let _source = EnvGuard::set("METRICS_SOURCE_ID", "raid-test");
        let _interval = EnvGuard::set("METRICS_INTERVAL_MS", "150");
        let _ops = EnvGuard::set("METRICS_OPS_PER_TICK", "42");
        let _queue = EnvGuard::set("METRICS_QUEUE_CAP", "64");
        let _conn = EnvGuard::set("METRICS_CONN_BUFFER", "7");
        let _connect = EnvGuard::set("METRICS_CONNECT_TIMEOUT_MS", "300");
        let _rpc = EnvGuard::set("METRICS_RPC_TIMEOUT_MS", "250");
        let _backoff_initial = EnvGuard::set("METRICS_BACKOFF_INITIAL_MS", "10");
        let _backoff_max = EnvGuard::set("METRICS_BACKOFF_MAX_MS", "900");
        let _jitter = EnvGuard::set("METRICS_JITTER_RATIO", "0.7");
        let _shutdown = EnvGuard::set("METRICS_SHUTDOWN_GRACE_MS", "800");
        let _auth = EnvGuard::set("GRPC_AUTH_TOKEN", "token");

        let cli = Cli::parse_from(["raid-cli", "metrics"]);
        let Command::Metrics(args) = cli.command else {
            panic!("expected metrics command");
        };

        assert_eq!(args.socket_path, "/tmp/metrics.sock");
        assert_eq!(args.source_id, "raid-test");
        assert_eq!(args.interval_ms, 150);
        assert_eq!(args.ops_per_tick, 42);
        assert_eq!(args.queue_cap, 64);
        assert_eq!(args.conn_buffer, 7);
        assert_eq!(args.connect_timeout_ms, 300);
        assert_eq!(args.rpc_timeout_ms, 250);
        assert_eq!(args.backoff_initial_ms, 10);
        assert_eq!(args.backoff_max_ms, 900);
        assert!((args.jitter_ratio - 0.7).abs() < f64::EPSILON);
        assert_eq!(args.shutdown_grace_ms, 800);
        assert_eq!(args.auth_token, "token");
    }

    #[test]
    fn parses_fuse_with_custom_raid_mode() {
        let cli = Cli::parse_from([
            "raid-cli",
            "fuse",
            "--mount-point",
            "/mnt/raid",
            "--disk-dir",
            "/var/raid",
            "--raid",
            "raid1",
            "--disks",
            "2",
            "--disk-size",
            "2048",
        ]);

        let Command::Fuse(args) = cli.command else {
            panic!("expected fuse command");
        };

        assert_eq!(args.raid, RaidMode::Raid1);
        assert_eq!(args.disks, 2);
        assert_eq!(args.disk_size, 2048);
    }
}
