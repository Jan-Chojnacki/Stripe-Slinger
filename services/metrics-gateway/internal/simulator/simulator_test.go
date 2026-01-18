package simulator

import (
	"context"
	"sync"
	"testing"
	"time"

	metricsPkg "metrics-gateway/internal/metrics"

	"github.com/prometheus/client_golang/prometheus/testutil"
)

func newTestSimulator(t *testing.T) (*Simulator, *metricsPkg.AllMetrics) {
	t.Helper()

	_, all := metricsPkg.NewMetricsRegistry()
	if all == nil {
		t.Fatal("expected non-nil metrics")
	}

	diskIDs := []string{"disk0", "disk1"}
	raidIDs := []string{"raid0", "raid1", "raid3"}

	sim := NewSimulator(all, diskIDs, raidIDs)
	if sim == nil {
		t.Fatal("expected non-nil simulator")
	}

	return sim, all
}

func TestSimulatorTickUpdatesDiskMetrics(t *testing.T) {
	sim, all := newTestSimulator(t)

	sim.tick()

	v := testutil.ToFloat64(all.Disks.ReadOps.WithLabelValues("disk0"))
	if v == 0 {
		t.Fatalf("expected disk read ops for disk0 to be > 0, got %f", v)
	}

	bytes := testutil.ToFloat64(all.Disks.ReadBytes.WithLabelValues("disk0"))
	if bytes == 0 {
		t.Fatalf("expected disk read bytes for disk0 to be > 0, got %f", bytes)
	}
}

func TestSimulatorTickUpdatesRaidFuseAndProcessMetrics(t *testing.T) {
	sim, all := newTestSimulator(t)

	sim.tick()

	if v := testutil.ToFloat64(all.Raid.ReadOps.WithLabelValues("raid0")); v == 0 {
		t.Fatalf("expected raid read ops for raid0 > 0, got %f", v)
	}
	if v := testutil.ToFloat64(all.Fuse.ReadOps); v == 0 {
		t.Fatalf("expected fuse read ops > 0, got %f", v)
	}
	if v := testutil.ToFloat64(all.Process.CPUSeconds); v == 0 {
		t.Fatalf("expected process CPU seconds > 0, got %f", v)
	}
	if v := testutil.ToFloat64(all.Process.ResidentMemory); v == 0 {
		t.Fatalf("expected process resident memory > 0, got %f", v)
	}
}

func TestSimulatorStartStopsOnContextCancel(t *testing.T) {
	sim, _ := newTestSimulator(t)

	ctx, cancel := context.WithCancel(context.Background())
	var wg sync.WaitGroup

	sim.Start(ctx, &wg, 10*time.Millisecond)

	time.Sleep(30 * time.Millisecond)

	cancel()

	done := make(chan struct{})
	go func() {
		wg.Wait()
		close(done)
	}()

	select {
	case <-done:
	case <-time.After(1 * time.Second):
		t.Fatal("simulator did not stop after context cancellation")
	}
}
