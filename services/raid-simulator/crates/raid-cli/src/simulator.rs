use std::time::{SystemTime, UNIX_EPOCH};

use prost_types::Timestamp;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rand_distr::{Distribution, Exp};

use crate::pb::metrics as pb;

pub struct SyntheticSimulator {
    rng: StdRng,
    disk_ids: Vec<String>,
    raid_ids: Vec<String>,
    exp_disk: Exp<f64>,
    exp_raid: Exp<f64>,
    exp_fuse: Exp<f64>,
    cpu_seconds: f64,
}

impl SyntheticSimulator {
    pub fn new(disk_ids: Vec<String>, raid_ids: Vec<String>) -> Self {
        let exp_disk = Exp::new(1.0 / 0.002).unwrap();
        let exp_raid = Exp::new(1.0 / 0.003).unwrap();
        let exp_fuse = Exp::new(1.0 / 0.0015).unwrap();

        Self {
            rng: StdRng::from_os_rng(),
            disk_ids,
            raid_ids,
            exp_disk,
            exp_raid,
            exp_fuse,
            cpu_seconds: 0.0,
        }
    }

    pub fn next_batch(
        &mut self,
        source_id: &str,
        seq_no: u64,
        ops_per_tick: u32,
    ) -> pb::MetricsBatch {
        let now = now_ts();

        let mut disk_ops = Vec::new();
        let mut disk_states = Vec::new();
        let mut raid_ops = Vec::new();
        let mut raid_states = Vec::new();
        let mut fuse_ops = Vec::new();

        for d in &self.disk_ids {
            disk_states.push(pb::DiskState {
                disk_id: d.clone(),
                queue_depth: self.rng.random_range(0.0..32.0),
            });
        }

        for r in &self.raid_ids {
            let degraded = self.rng.random_bool(0.005);
            let failed = if degraded {
                self.rng.random_range(1..=2)
            } else {
                0
            };
            let rebuild = degraded && self.rng.random_bool(0.3);

            let raid1_resync = if r == "raid1" {
                self.rng.random_range(0.0..=1.0)
            } else {
                0.0
            };

            raid_states.push(pb::RaidState {
                raid_id: r.clone(),
                raid1_resync_progress: raid1_resync,
                degraded,
                failed_disks: failed,
                rebuild_in_progress: rebuild,
            });
        }

        let per = ops_per_tick.max(1) as usize;
        for _ in 0..per {
            {
                let disk_id = self.pick_disk().to_string();
                let is_read = self.rng.random_bool(0.55);
                let bytes = self.pick_bytes();
                let latency = self.sample_disk_latency(0.050);
                let error = self.rng.random_bool(0.001);

                disk_ops.push(pb::DiskOp {
                    disk_id,
                    op: if is_read {
                        pb::IoOpType::IoOpRead as i32
                    } else {
                        pb::IoOpType::IoOpWrite as i32
                    },
                    bytes,
                    latency_seconds: latency,
                    error,
                });
            }

            {
                let raid_id = self.pick_raid().to_string();
                let is_read = self.rng.random_bool(0.50);
                let bytes = self.pick_bytes();
                let latency = self.sample_raid_latency(0.080);
                let error = self.rng.random_bool(0.001);

                let served_from_disk_id =
                    if raid_id == "raid1" && is_read && self.rng.random_bool(0.7) {
                        self.pick_disk().to_string()
                    } else {
                        String::new()
                    };

                let (parity_r, parity_w, partial_w) = if raid_id == "raid3" {
                    (
                        is_read && self.rng.random_bool(0.2),
                        !is_read && self.rng.random_bool(0.25),
                        !is_read && self.rng.random_bool(0.15),
                    )
                } else {
                    (false, false, false)
                };

                raid_ops.push(pb::RaidOp {
                    raid_id,
                    op: if is_read {
                        pb::IoOpType::IoOpRead as i32
                    } else {
                        pb::IoOpType::IoOpWrite as i32
                    },
                    bytes,
                    latency_seconds: latency,
                    error,
                    served_from_disk_id,
                    raid3_parity_read: parity_r,
                    raid3_parity_write: parity_w,
                    raid3_partial_stripe_write: partial_w,
                });
            }

            {
                let roll: f64 = self.rng.random();
                let (op, bytes, latency) = if roll < 0.45 {
                    (
                        pb::FuseOpType::FuseOpRead,
                        self.pick_bytes(),
                        self.sample_fuse_latency(0.030),
                    )
                } else if roll < 0.90 {
                    (
                        pb::FuseOpType::FuseOpWrite,
                        self.pick_bytes(),
                        self.sample_fuse_latency(0.030),
                    )
                } else if roll < 0.97 {
                    (pb::FuseOpType::FuseOpOpen, 0, 0.0)
                } else {
                    (pb::FuseOpType::FuseOpFsync, 0, 0.0)
                };

                let error = self.rng.random_bool(0.0005);

                fuse_ops.push(pb::FuseOp {
                    op: op as i32,
                    bytes,
                    latency_seconds: latency,
                    error,
                });
            }
        }

        self.cpu_seconds += self.rng.random_range(0.001..0.05);
        let resident_memory = self.rng.random_range(150_000_000u64..450_000_000u64);

        let process = Some(pb::ProcessSample {
            cpu_seconds: self.cpu_seconds,
            resident_memory_bytes: resident_memory,
        });

        pb::MetricsBatch {
            source_id: source_id.to_string(),
            seq_no,
            timestamp: Some(now),
            disk_ops,
            disk_states,
            raid_ops,
            raid_states,
            fuse_ops,
            process,
        }
    }

    fn pick_disk(&mut self) -> &str {
        let i = self.rng.random_range(0..self.disk_ids.len());
        &self.disk_ids[i]
    }

    fn pick_raid(&mut self) -> &str {
        let i = self.rng.random_range(0..self.raid_ids.len());
        &self.raid_ids[i]
    }

    fn pick_bytes(&mut self) -> u64 {
        let choices = [4096u64, 8192, 16384, 32768, 65536, 131072, 262144];
        let i = self.rng.random_range(0..choices.len());
        choices[i]
    }

    fn sample_disk_latency(&mut self, cap_seconds: f64) -> f64 {
        let v = self.exp_disk.sample(&mut self.rng);
        v.min(cap_seconds).max(0.0)
    }

    fn sample_raid_latency(&mut self, cap_seconds: f64) -> f64 {
        let v = self.exp_raid.sample(&mut self.rng);
        v.min(cap_seconds).max(0.0)
    }

    fn sample_fuse_latency(&mut self, cap_seconds: f64) -> f64 {
        let v = self.exp_fuse.sample(&mut self.rng);
        v.min(cap_seconds).max(0.0)
    }
}

fn now_ts() -> Timestamp {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp {
        seconds: dur.as_secs() as i64,
        nanos: dur.subsec_nanos() as i32,
    }
}
