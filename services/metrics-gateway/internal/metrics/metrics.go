package metrics

import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/collectors"
)

type DiskMetrics struct {
	ReadOps      *prometheus.CounterVec
	WriteOps     *prometheus.CounterVec
	ReadBytes    *prometheus.CounterVec
	WriteBytes   *prometheus.CounterVec
	ReadLatency  *prometheus.GaugeVec
	WriteLatency *prometheus.GaugeVec
	QueueDepth   *prometheus.GaugeVec
	Errors       *prometheus.CounterVec
}

type RaidMetrics struct {
	ReadOps            *prometheus.CounterVec
	WriteOps           *prometheus.CounterVec
	ReadBytes          *prometheus.CounterVec
	WriteBytes         *prometheus.CounterVec
	ReadLatency        *prometheus.GaugeVec
	WriteLatency       *prometheus.GaugeVec
	Raid1ReadsFromDisk *prometheus.CounterVec
	Raid1Resync        *prometheus.GaugeVec
	Raid3ParityReads   *prometheus.CounterVec
	Raid3ParityWrites  *prometheus.CounterVec
	Raid3PartialStripe *prometheus.CounterVec
	DegradedState      *prometheus.GaugeVec
	FailedDisks        *prometheus.GaugeVec
	RebuildInProgress  *prometheus.GaugeVec
}

type FuseMetrics struct {
	ReadOps      prometheus.Counter
	WriteOps     prometheus.Counter
	OpenOps      prometheus.Counter
	FsyncOps     prometheus.Counter
	ReadBytes    prometheus.Counter
	WriteBytes   prometheus.Counter
	ReadLatency  prometheus.Gauge
	WriteLatency prometheus.Gauge
	Errors       prometheus.Counter
}

type ProcessMetrics struct {
	CPUSeconds     prometheus.Gauge
	ResidentMemory prometheus.Gauge
}

type AllMetrics struct {
	Disks   *DiskMetrics
	Raid    *RaidMetrics
	Fuse    *FuseMetrics
	Process *ProcessMetrics
}

func NewMetricsRegistry() (*prometheus.Registry, *AllMetrics) {
	reg := prometheus.NewRegistry()

	reg.MustRegister(
		collectors.NewGoCollector(),
		collectors.NewProcessCollector(collectors.ProcessCollectorOpts{}),
	)

	all := &AllMetrics{
		Disks:   NewDiskMetrics(reg),
		Raid:    NewRaidMetrics(reg),
		Fuse:    NewFuseMetrics(reg),
		Process: NewProcessMetrics(reg),
	}

	return reg, all
}

func newCounterVec(reg prometheus.Registerer, name, help string, labels ...string) *prometheus.CounterVec {
	cv := prometheus.NewCounterVec(
		prometheus.CounterOpts{
			Name: name,
			Help: help,
		},
		labels,
	)
	reg.MustRegister(cv)
	return cv
}

func newGaugeVec(reg prometheus.Registerer, name, help string, labels ...string) *prometheus.GaugeVec {
	gv := prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: name,
			Help: help,
		},
		labels,
	)
	reg.MustRegister(gv)
	return gv
}

func newCounter(reg prometheus.Registerer, name, help string) prometheus.Counter {
	c := prometheus.NewCounter(
		prometheus.CounterOpts{
			Name: name,
			Help: help,
		},
	)
	reg.MustRegister(c)
	return c
}

func newGauge(reg prometheus.Registerer, name, help string) prometheus.Gauge {
	g := prometheus.NewGauge(
		prometheus.GaugeOpts{
			Name: name,
			Help: help,
		},
	)
	reg.MustRegister(g)
	return g
}

func NewDiskMetrics(reg prometheus.Registerer) *DiskMetrics {
	return &DiskMetrics{
		ReadOps:      newCounterVec(reg, "disk_read_ops", "Number of disk read operations", "disk_id"),
		WriteOps:     newCounterVec(reg, "disk_write_ops", "Number of disk write operations", "disk_id"),
		ReadBytes:    newCounterVec(reg, "disk_read_bytes", "Bytes read from disk", "disk_id"),
		WriteBytes:   newCounterVec(reg, "disk_write_bytes", "Bytes written to disk", "disk_id"),
		ReadLatency:  newGaugeVec(reg, "disk_read_latency", "Average disk read latency (seconds)", "disk_id"),
		WriteLatency: newGaugeVec(reg, "disk_write_latency", "Average disk write latency (seconds)", "disk_id"),
		QueueDepth:   newGaugeVec(reg, "disk_queue_depth", "Current disk queue depth", "disk_id"),
		Errors:       newCounterVec(reg, "disk_errors", "Total disk errors", "disk_id"),
	}
}

func NewRaidMetrics(reg prometheus.Registerer) *RaidMetrics {
	return &RaidMetrics{
		ReadOps:            newCounterVec(reg, "raid_read_ops", "Total RAID read operations", "raid"),
		WriteOps:           newCounterVec(reg, "raid_write_ops", "Total RAID write operations", "raid"),
		ReadBytes:          newCounterVec(reg, "raid_read_bytes", "Total RAID read bytes", "raid"),
		WriteBytes:         newCounterVec(reg, "raid_write_bytes", "Total RAID write bytes", "raid"),
		ReadLatency:        newGaugeVec(reg, "raid_read_latency", "Average RAID read latency (seconds)", "raid"),
		WriteLatency:       newGaugeVec(reg, "raid_write_latency", "Average RAID write latency (seconds)", "raid"),
		Raid1ReadsFromDisk: newCounterVec(reg, "raid1_reads_from_disk", "Reads served from a given disk in RAID1", "raid", "disk_id"),
		Raid1Resync:        newGaugeVec(reg, "raid1_resync_progress", "RAID1 resync progress (0-1)", "raid"),
		Raid3ParityReads:   newCounterVec(reg, "raid3_parity_reads", "RAID3 parity read operations", "raid"),
		Raid3ParityWrites:  newCounterVec(reg, "raid3_parity_writes", "RAID3 parity write operations", "raid"),
		Raid3PartialStripe: newCounterVec(reg, "raid3_partial_stripe_writes", "RAID3 partial stripe writes", "raid"),
		DegradedState:      newGaugeVec(reg, "raid_degraded_state", "RAID degraded state (0/1)", "raid"),
		FailedDisks:        newGaugeVec(reg, "raid_failed_disks", "Number of failed disks in RAID", "raid"),
		RebuildInProgress:  newGaugeVec(reg, "raid_rebuild_in_progress", "RAID rebuild in progress (0/1)", "raid"),
	}
}

func NewFuseMetrics(reg prometheus.Registerer) *FuseMetrics {
	return &FuseMetrics{
		ReadOps:      newCounter(reg, "fuse_read_ops", "Number of FUSE read operations"),
		WriteOps:     newCounter(reg, "fuse_write_ops", "Number of FUSE write operations"),
		OpenOps:      newCounter(reg, "fuse_open_ops", "Number of FUSE open operations"),
		FsyncOps:     newCounter(reg, "fuse_fsync_ops", "Number of FUSE fsync operations"),
		ReadBytes:    newCounter(reg, "fuse_read_bytes", "Bytes read via FUSE"),
		WriteBytes:   newCounter(reg, "fuse_write_bytes", "Bytes written via FUSE"),
		ReadLatency:  newGauge(reg, "fuse_read_latency", "Average FUSE read latency (seconds)"),
		WriteLatency: newGauge(reg, "fuse_write_latency", "Average FUSE write latency (seconds)"),
		Errors:       newCounter(reg, "fuse_errors", "Total FUSE errors"),
	}
}

func NewProcessMetrics(reg prometheus.Registerer) *ProcessMetrics {
	return &ProcessMetrics{
		CPUSeconds:     newGauge(reg, "process_cpu_seconds", "Simulated CPU seconds used by the RAID simulator"),
		ResidentMemory: newGauge(reg, "process_resident_memory", "Simulated resident memory (bytes)"),
	}
}
