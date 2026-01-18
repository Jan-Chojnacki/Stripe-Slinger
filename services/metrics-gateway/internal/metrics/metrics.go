package metrics

import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/collectors"
)

var defaultLatencyBuckets = []float64{
	0.00005, 0.0001, 0.00025, 0.0005,
	0.001, 0.0025, 0.005, 0.01,
	0.025, 0.05, 0.1, 0.25,
	0.5, 1, 2.5, 5, 10,
}

// DiskMetrics bundles Prometheus metrics tracking disk IO behavior.
type DiskMetrics struct {
	ReadOps      *prometheus.CounterVec
	WriteOps     *prometheus.CounterVec
	ReadBytes    *prometheus.CounterVec
	WriteBytes   *prometheus.CounterVec
	ReadLatency  *prometheus.HistogramVec
	WriteLatency *prometheus.HistogramVec
	QueueDepth   *prometheus.GaugeVec
	Errors       *prometheus.CounterVec
}

// RaidMetrics bundles Prometheus metrics tracking RAID volume behavior.
type RaidMetrics struct {
	ReadOps            *prometheus.CounterVec
	WriteOps           *prometheus.CounterVec
	ReadBytes          *prometheus.CounterVec
	WriteBytes         *prometheus.CounterVec
	ReadLatency        *prometheus.HistogramVec
	WriteLatency       *prometheus.HistogramVec
	Raid1ReadsFromDisk *prometheus.CounterVec
	Raid1Resync        *prometheus.GaugeVec
	Raid3ParityReads   *prometheus.CounterVec
	Raid3ParityWrites  *prometheus.CounterVec
	Raid3PartialStripe *prometheus.CounterVec
	DegradedState      *prometheus.GaugeVec
	FailedDisks        *prometheus.GaugeVec
	RebuildInProgress  *prometheus.GaugeVec
}

// FuseMetrics bundles Prometheus metrics tracking FUSE-level operations.
type FuseMetrics struct {
	ReadOps      prometheus.Counter
	WriteOps     prometheus.Counter
	OpenOps      prometheus.Counter
	FsyncOps     prometheus.Counter
	ReadBytes    prometheus.Counter
	WriteBytes   prometheus.Counter
	ReadLatency  prometheus.Histogram
	WriteLatency prometheus.Histogram
	Errors       prometheus.Counter
}

// ProcessMetrics bundles Prometheus gauges tracking simulated process usage.
type ProcessMetrics struct {
	CPUSeconds     prometheus.Gauge
	ResidentMemory prometheus.Gauge
}

// AllMetrics aggregates all metric families used by the gateway.
type AllMetrics struct {
	Disks   *DiskMetrics
	Raid    *RaidMetrics
	Fuse    *FuseMetrics
	Process *ProcessMetrics
}

// NewMetricsRegistry creates a registry and registers all metrics used by the gateway.
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
		prometheus.CounterOpts{Name: name, Help: help},
		labels,
	)
	reg.MustRegister(cv)
	return cv
}

func newGaugeVec(reg prometheus.Registerer, name, help string, labels ...string) *prometheus.GaugeVec {
	gv := prometheus.NewGaugeVec(
		prometheus.GaugeOpts{Name: name, Help: help},
		labels,
	)
	reg.MustRegister(gv)
	return gv
}

func newHistogramVec(reg prometheus.Registerer, name, help string, buckets []float64, labels ...string) *prometheus.HistogramVec {
	hv := prometheus.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    name,
			Help:    help,
			Buckets: buckets,
		},
		labels,
	)
	reg.MustRegister(hv)
	return hv
}

func newCounter(reg prometheus.Registerer, name, help string) prometheus.Counter {
	c := prometheus.NewCounter(prometheus.CounterOpts{Name: name, Help: help})
	reg.MustRegister(c)
	return c
}

func newGauge(reg prometheus.Registerer, name, help string) prometheus.Gauge {
	g := prometheus.NewGauge(prometheus.GaugeOpts{Name: name, Help: help})
	reg.MustRegister(g)
	return g
}

func newHistogram(reg prometheus.Registerer, name, help string, buckets []float64) prometheus.Histogram {
	h := prometheus.NewHistogram(prometheus.HistogramOpts{Name: name, Help: help, Buckets: buckets})
	reg.MustRegister(h)
	return h
}

// NewDiskMetrics registers disk metrics with the provided registry.
func NewDiskMetrics(reg prometheus.Registerer) *DiskMetrics {
	return &DiskMetrics{
		ReadOps:    newCounterVec(reg, "disk_read_ops", "Number of disk read operations", "disk_id"),
		WriteOps:   newCounterVec(reg, "disk_write_ops", "Number of disk write operations", "disk_id"),
		ReadBytes:  newCounterVec(reg, "disk_read_bytes", "Bytes read from disk", "disk_id"),
		WriteBytes: newCounterVec(reg, "disk_write_bytes", "Bytes written to disk", "disk_id"),
		ReadLatency: newHistogramVec(
			reg,
			"disk_read_latency_seconds",
			"Disk read latency (seconds)",
			defaultLatencyBuckets,
			"disk_id",
		),
		WriteLatency: newHistogramVec(
			reg,
			"disk_write_latency_seconds",
			"Disk write latency (seconds)",
			defaultLatencyBuckets,
			"disk_id",
		),
		QueueDepth: newGaugeVec(reg, "disk_queue_depth", "Current disk queue depth", "disk_id"),
		Errors:     newCounterVec(reg, "disk_errors", "Total disk errors", "disk_id"),
	}
}

// NewRaidMetrics registers RAID metrics with the provided registry.
func NewRaidMetrics(reg prometheus.Registerer) *RaidMetrics {
	return &RaidMetrics{
		ReadOps:    newCounterVec(reg, "raid_read_ops", "Total RAID read operations", "raid"),
		WriteOps:   newCounterVec(reg, "raid_write_ops", "Total RAID write operations", "raid"),
		ReadBytes:  newCounterVec(reg, "raid_read_bytes", "Total RAID read bytes", "raid"),
		WriteBytes: newCounterVec(reg, "raid_write_bytes", "Total RAID write bytes", "raid"),
		ReadLatency: newHistogramVec(
			reg,
			"raid_read_latency_seconds",
			"RAID read latency (seconds)",
			defaultLatencyBuckets,
			"raid",
		),
		WriteLatency: newHistogramVec(
			reg,
			"raid_write_latency_seconds",
			"RAID write latency (seconds)",
			defaultLatencyBuckets,
			"raid",
		),
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

// NewFuseMetrics registers FUSE metrics with the provided registry.
func NewFuseMetrics(reg prometheus.Registerer) *FuseMetrics {
	return &FuseMetrics{
		ReadOps:      newCounter(reg, "fuse_read_ops", "Number of FUSE read operations"),
		WriteOps:     newCounter(reg, "fuse_write_ops", "Number of FUSE write operations"),
		OpenOps:      newCounter(reg, "fuse_open_ops", "Number of FUSE open operations"),
		FsyncOps:     newCounter(reg, "fuse_fsync_ops", "Number of FUSE fsync operations"),
		ReadBytes:    newCounter(reg, "fuse_read_bytes", "Bytes read via FUSE"),
		WriteBytes:   newCounter(reg, "fuse_write_bytes", "Bytes written via FUSE"),
		ReadLatency:  newHistogram(reg, "fuse_read_latency_seconds", "FUSE read latency (seconds)", defaultLatencyBuckets),
		WriteLatency: newHistogram(reg, "fuse_write_latency_seconds", "FUSE write latency (seconds)", defaultLatencyBuckets),
		Errors:       newCounter(reg, "fuse_errors", "Total FUSE errors"),
	}
}

// NewProcessMetrics registers process metrics with the provided registry.
func NewProcessMetrics(reg prometheus.Registerer) *ProcessMetrics {
	return &ProcessMetrics{
		CPUSeconds:     newGauge(reg, "process_cpu_seconds", "Simulated CPU seconds used by the RAID simulator"),
		ResidentMemory: newGauge(reg, "process_resident_memory", "Simulated resident memory (bytes)"),
	}
}
