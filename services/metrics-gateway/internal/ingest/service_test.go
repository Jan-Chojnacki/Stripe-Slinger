package ingest

import (
	"context"
	"io"
	"math"
	"testing"

	"metrics-gateway/internal/metrics"
	pb "metrics-gateway/internal/pb/metrics/v1"

	"github.com/prometheus/client_golang/prometheus/testutil"
)

func newTestService(t *testing.T) *Service {
	t.Helper()

	_, all := metrics.NewMetricsRegistry()
	if all == nil {
		t.Fatal("expected non-nil metrics")
	}

	return NewService(all)
}

func TestApplyDiskOpReadUpdatesMetrics(t *testing.T) {
	svc := newTestService(t)

	op := &pb.DiskOp{
		DiskId:         "disk0",
		Op:             pb.IoOpType_IO_OP_READ,
		Bytes:          2048,
		LatencySeconds: 0.5,
		Error:          true,
	}

	if ok := svc.applyDiskOp(op); !ok {
		t.Fatal("expected applyDiskOp to succeed")
	}

	if v := testutil.ToFloat64(svc.m.Disks.ReadOps.WithLabelValues("disk0")); v != 1 {
		t.Fatalf("expected read ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Disks.ReadBytes.WithLabelValues("disk0")); v != 2048 {
		t.Fatalf("expected read bytes to be 2048, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Disks.Errors.WithLabelValues("disk0")); v != 1 {
		t.Fatalf("expected errors to be 1, got %f", v)
	}
}

func TestApplyRaidWriteUpdatesMetrics(t *testing.T) {
	svc := newTestService(t)

	op := &pb.RaidOp{
		RaidId:                  "raid3",
		Op:                      pb.IoOpType_IO_OP_WRITE,
		Bytes:                   4096,
		LatencySeconds:          0.25,
		Raid3ParityWrite:        true,
		Raid3PartialStripeWrite: true,
	}

	if ok := svc.applyRaidOp(op); !ok {
		t.Fatal("expected applyRaidOp to succeed")
	}

	if v := testutil.ToFloat64(svc.m.Raid.WriteOps.WithLabelValues("raid3")); v != 1 {
		t.Fatalf("expected raid write ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.WriteBytes.WithLabelValues("raid3")); v != 4096 {
		t.Fatalf("expected raid write bytes to be 4096, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.Raid3ParityWrites.WithLabelValues("raid3")); v != 1 {
		t.Fatalf("expected raid3 parity writes to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.Raid3PartialStripe.WithLabelValues("raid3")); v != 1 {
		t.Fatalf("expected raid3 partial stripe writes to be 1, got %f", v)
	}
}

func TestHandleProcessTracksAcceptReject(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleProcess(&pb.ProcessSample{
		CpuSeconds:          math.NaN(),
		ResidentMemoryBytes: 256,
	}, counters)

	if counters.acceptedSamples != 1 {
		t.Fatalf("expected accepted samples to be 1, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 1 {
		t.Fatalf("expected rejected samples to be 1, got %d", counters.rejectedSamples)
	}

	if v := testutil.ToFloat64(svc.m.Process.ResidentMemory); v != 256 {
		t.Fatalf("expected resident memory to be 256, got %f", v)
	}
}

func TestHandleDiskOpsTracksAcceptReject(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleDiskOps([]*pb.DiskOp{
		{
			DiskId:         "disk-ok",
			Op:             pb.IoOpType_IO_OP_WRITE,
			Bytes:          128,
			LatencySeconds: 0.01,
		},
		{
			DiskId:         "bad id",
			Op:             pb.IoOpType_IO_OP_READ,
			Bytes:          64,
			LatencySeconds: 0.01,
		},
		{
			DiskId:         "disk-ok",
			Op:             pb.IoOpType_IO_OP_UNSPECIFIED,
			Bytes:          64,
			LatencySeconds: 0.01,
		},
	}, counters)

	if counters.acceptedSamples != 1 {
		t.Fatalf("expected accepted samples to be 1, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 2 {
		t.Fatalf("expected rejected samples to be 2, got %d", counters.rejectedSamples)
	}

	if v := testutil.ToFloat64(svc.m.Disks.WriteOps.WithLabelValues("disk-ok")); v != 1 {
		t.Fatalf("expected write ops for disk-ok to be 1, got %f", v)
	}
}

func TestHandleFuseOpsRejectsUnspecifiedOp(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleFuseOps([]*pb.FuseOp{
		{Op: pb.FuseOpType_FUSE_OP_UNSPECIFIED},
	}, counters)

	if counters.acceptedSamples != 0 {
		t.Fatalf("expected accepted samples to be 0, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 1 {
		t.Fatalf("expected rejected samples to be 1, got %d", counters.rejectedSamples)
	}
}

func TestValidateFunctions(t *testing.T) {
	if !validateDiskOp(&pb.DiskOp{DiskId: "disk0", LatencySeconds: 0.1}) {
		t.Fatal("expected disk op to be valid")
	}
	if validateDiskOp(&pb.DiskOp{DiskId: "disk0", LatencySeconds: -1}) {
		t.Fatal("expected disk op to reject negative latency")
	}
	if validateDiskState(&pb.DiskState{DiskId: "disk0", QueueDepth: 1}) != true {
		t.Fatal("expected disk state to be valid")
	}
	if validateDiskState(&pb.DiskState{DiskId: "disk0", QueueDepth: math.NaN()}) {
		t.Fatal("expected disk state to reject NaN queue depth")
	}
	if !validateRaidOp(&pb.RaidOp{RaidId: "raid1", LatencySeconds: 0.1}) {
		t.Fatal("expected raid op to be valid")
	}
	if validateRaidState(&pb.RaidState{RaidId: "raid1", Raid1ResyncProgress: -0.5}) {
		t.Fatal("expected raid state to reject negative resync progress")
	}
	if !validateFuseOp(&pb.FuseOp{LatencySeconds: 0}) {
		t.Fatal("expected fuse op to be valid")
	}
	if validateFuseOp(&pb.FuseOp{LatencySeconds: math.Inf(1)}) {
		t.Fatal("expected fuse op to reject inf latency")
	}
}

func TestSetGaugeBool(t *testing.T) {
	svc := newTestService(t)
	gauge := svc.m.Raid.DegradedState.WithLabelValues("raid0")

	setGaugeBool(gauge, true)
	if v := testutil.ToFloat64(gauge); v != 1 {
		t.Fatalf("expected gauge to be 1, got %f", v)
	}

	setGaugeBool(gauge, false)
	if v := testutil.ToFloat64(gauge); v != 0 {
		t.Fatalf("expected gauge to be 0, got %f", v)
	}
}

func TestHandleDiskStatesAppliesQueueDepth(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleDiskStates([]*pb.DiskState{
		{DiskId: "disk0", QueueDepth: 7},
		{DiskId: "disk0", QueueDepth: -1},
	}, counters)

	if counters.acceptedSamples != 1 {
		t.Fatalf("expected accepted samples to be 1, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 1 {
		t.Fatalf("expected rejected samples to be 1, got %d", counters.rejectedSamples)
	}

	if v := testutil.ToFloat64(svc.m.Disks.QueueDepth.WithLabelValues("disk0")); v != 7 {
		t.Fatalf("expected queue depth to be 7, got %f", v)
	}
}

func TestHandleRaidStatesAppliesHealth(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleRaidStates([]*pb.RaidState{
		{
			RaidId:              "raid1",
			Raid1ResyncProgress: 0.5,
			Degraded:            true,
			FailedDisks:         2,
			RebuildInProgress:   true,
		},
		{
			RaidId:              "raid1",
			Raid1ResyncProgress: math.Inf(1),
		},
	}, counters)

	if counters.acceptedSamples != 1 {
		t.Fatalf("expected accepted samples to be 1, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 1 {
		t.Fatalf("expected rejected samples to be 1, got %d", counters.rejectedSamples)
	}

	if v := testutil.ToFloat64(svc.m.Raid.Raid1Resync.WithLabelValues("raid1")); v != 0.5 {
		t.Fatalf("expected resync to be 0.5, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.DegradedState.WithLabelValues("raid1")); v != 1 {
		t.Fatalf("expected degraded to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.FailedDisks.WithLabelValues("raid1")); v != 2 {
		t.Fatalf("expected failed disks to be 2, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.RebuildInProgress.WithLabelValues("raid1")); v != 1 {
		t.Fatalf("expected rebuild in progress to be 1, got %f", v)
	}
}

func TestHandleFuseOpsTracksAllOps(t *testing.T) {
	svc := newTestService(t)
	counters := &pushCounters{}

	svc.handleFuseOps([]*pb.FuseOp{
		{Op: pb.FuseOpType_FUSE_OP_READ, Bytes: 10, LatencySeconds: 0.1},
		{Op: pb.FuseOpType_FUSE_OP_WRITE, Bytes: 20, LatencySeconds: 0.2, Error: true},
		{Op: pb.FuseOpType_FUSE_OP_OPEN},
		{Op: pb.FuseOpType_FUSE_OP_FSYNC},
	}, counters)

	if counters.acceptedSamples != 4 {
		t.Fatalf("expected accepted samples to be 4, got %d", counters.acceptedSamples)
	}
	if counters.rejectedSamples != 0 {
		t.Fatalf("expected rejected samples to be 0, got %d", counters.rejectedSamples)
	}

	if v := testutil.ToFloat64(svc.m.Fuse.ReadOps); v != 1 {
		t.Fatalf("expected read ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Fuse.WriteOps); v != 1 {
		t.Fatalf("expected write ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Fuse.OpenOps); v != 1 {
		t.Fatalf("expected open ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Fuse.FsyncOps); v != 1 {
		t.Fatalf("expected fsync ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Fuse.Errors); v != 1 {
		t.Fatalf("expected fuse errors to be 1, got %f", v)
	}
}

func TestApplyRaidReadTracksExtras(t *testing.T) {
	svc := newTestService(t)

	op := &pb.RaidOp{
		RaidId:           "raid1",
		Op:               pb.IoOpType_IO_OP_READ,
		Bytes:            512,
		LatencySeconds:   0.1,
		ServedFromDiskId: "disk0",
		Raid3ParityRead:  true,
	}

	if ok := svc.applyRaidOp(op); !ok {
		t.Fatal("expected applyRaidOp to succeed")
	}

	if v := testutil.ToFloat64(svc.m.Raid.ReadOps.WithLabelValues("raid1")); v != 1 {
		t.Fatalf("expected raid read ops to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.Raid1ReadsFromDisk.WithLabelValues("raid1", "disk0")); v != 1 {
		t.Fatalf("expected raid1 reads from disk to be 1, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Raid.Raid3ParityReads.WithLabelValues("raid1")); v != 1 {
		t.Fatalf("expected parity reads to be 1, got %f", v)
	}
}

func TestRecordIOAndHelpers(t *testing.T) {
	svc := newTestService(t)

	recordIO(svc.m.Fuse.ReadOps, svc.m.Fuse.ReadBytes, svc.m.Fuse.ReadLatency, 12, -1)
	recordIO(svc.m.Fuse.ReadOps, svc.m.Fuse.ReadBytes, svc.m.Fuse.ReadLatency, 13, 0.5)

	if v := testutil.ToFloat64(svc.m.Fuse.ReadOps); v != 2 {
		t.Fatalf("expected read ops to be 2, got %f", v)
	}
	if v := testutil.ToFloat64(svc.m.Fuse.ReadBytes); v != 25 {
		t.Fatalf("expected read bytes to be 25, got %f", v)
	}

	if !validID("disk_1") {
		t.Fatal("expected id to be valid")
	}
	if validID("bad id") {
		t.Fatal("expected id to be invalid")
	}
	if !finiteNonNeg(0) {
		t.Fatal("expected 0 to be finite non-negative")
	}
	if finiteNonNeg(math.Inf(1)) {
		t.Fatal("expected +inf to be invalid")
	}
}

type fakePushStream struct {
	pb.MetricsIngestor_PushServer
	batches  []*pb.MetricsBatch
	recvIdx  int
	response *pb.PushResponse
}

func (f *fakePushStream) Recv() (*pb.MetricsBatch, error) {
	if f.recvIdx >= len(f.batches) {
		return nil, io.EOF
	}
	b := f.batches[f.recvIdx]
	f.recvIdx++
	return b, nil
}

func (f *fakePushStream) SendAndClose(resp *pb.PushResponse) error {
	f.response = resp
	return nil
}

func (f *fakePushStream) Context() context.Context {
	return context.Background()
}

func TestPushAggregatesCounters(t *testing.T) {
	svc := newTestService(t)

	stream := &fakePushStream{
		batches: []*pb.MetricsBatch{
			{
				SourceId: "",
				DiskOps: []*pb.DiskOp{
					{DiskId: "disk0", Op: pb.IoOpType_IO_OP_READ, Bytes: 10, LatencySeconds: 0.1},
				},
			},
			{
				SourceId: "src",
				DiskOps: []*pb.DiskOp{
					{DiskId: "disk0", Op: pb.IoOpType_IO_OP_READ, Bytes: 10, LatencySeconds: 0.1},
					{DiskId: "disk0", Op: pb.IoOpType_IO_OP_UNSPECIFIED, Bytes: 10, LatencySeconds: 0.1},
				},
				DiskStates: []*pb.DiskState{
					{DiskId: "disk0", QueueDepth: 4},
				},
				RaidOps: []*pb.RaidOp{
					{RaidId: "raid1", Op: pb.IoOpType_IO_OP_READ, Bytes: 5, LatencySeconds: 0.1},
				},
				RaidStates: []*pb.RaidState{
					{RaidId: "raid1", Raid1ResyncProgress: 0.3},
				},
				FuseOps: []*pb.FuseOp{
					{Op: pb.FuseOpType_FUSE_OP_OPEN},
				},
				Process: &pb.ProcessSample{
					CpuSeconds:          1.5,
					ResidentMemoryBytes: 128,
				},
			},
		},
	}

	if err := svc.Push(stream); err != nil {
		t.Fatalf("expected push to succeed: %v", err)
	}
	if stream.response == nil {
		t.Fatal("expected response to be sent")
	}
	if stream.response.AcceptedBatches != 1 {
		t.Fatalf("expected accepted batches to be 1, got %d", stream.response.AcceptedBatches)
	}
	if stream.response.AcceptedSamples == 0 {
		t.Fatal("expected accepted samples to be > 0")
	}
	if stream.response.RejectedSamples == 0 {
		t.Fatal("expected rejected samples to be > 0")
	}
}
