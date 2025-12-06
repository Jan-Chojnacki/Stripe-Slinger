package metrics

import (
	"testing"
)

func TestNewMetricsRegistryInitializesAllGroups(t *testing.T) {
	reg, all := NewMetricsRegistry()

	if reg == nil {
		t.Fatal("expected non-nil registry")
	}
	if all == nil {
		t.Fatal("expected non-nil AllMetrics")
	}

	if all.Disks == nil {
		t.Fatal("expected disk metrics to be initialized")
	}
	if all.Raid == nil {
		t.Fatal("expected raid metrics to be initialized")
	}
	if all.Fuse == nil {
		t.Fatal("expected fuse metrics to be initialized")
	}
	if all.Process == nil {
		t.Fatal("expected process metrics to be initialized")
	}

	all.Disks.ReadOps.WithLabelValues("disk0").Inc()
	all.Disks.WriteOps.WithLabelValues("disk0").Add(5)
	all.Raid.ReadOps.WithLabelValues("raid0").Add(10)
	all.Fuse.ReadOps.Inc()
	all.Process.CPUSeconds.Set(1.23)

	if _, err := reg.Gather(); err != nil {
		t.Fatalf("gather on registry failed: %v", err)
	}
}
