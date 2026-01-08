package ingest

import (
	"io"
	"math"
	"regexp"

	"metrics-gateway/internal/metrics"
	pb "metrics-gateway/internal/pb/metrics/v1"

	"github.com/prometheus/client_golang/prometheus"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

var idRe = regexp.MustCompile(`^[a-zA-Z0-9_-]{1,64}$`)

type Service struct {
	pb.UnimplementedMetricsIngestorServer
	m *metrics.AllMetrics
}

func NewService(m *metrics.AllMetrics) *Service {
	return &Service{m: m}
}

type pushCounters struct {
	acceptedBatches uint64
	acceptedSamples uint64
	rejectedSamples uint64
}

func (c *pushCounters) acceptSample() { c.acceptedSamples++ }
func (c *pushCounters) rejectSample() { c.rejectedSamples++ }

func (s *Service) Push(stream pb.MetricsIngestor_PushServer) error {
	var c pushCounters

	for {
		batch, err := stream.Recv()
		if err == io.EOF {
			return stream.SendAndClose(&pb.PushResponse{
				AcceptedBatches: c.acceptedBatches,
				AcceptedSamples: c.acceptedSamples,
				RejectedSamples: c.rejectedSamples,
			})
		}
		if err != nil {
			return status.Errorf(codes.Unknown, "recv batch: %v", err)
		}

		if batch.GetSourceId() == "" {
			c.rejectSample()
			continue
		}

		c.acceptedBatches++

		s.handleDiskOps(batch.GetDiskOps(), &c)
		s.handleDiskStates(batch.GetDiskStates(), &c)
		s.handleRaidOps(batch.GetRaidOps(), &c)
		s.handleRaidStates(batch.GetRaidStates(), &c)
		s.handleFuseOps(batch.GetFuseOps(), &c)
		s.handleProcess(batch.GetProcess(), &c)
	}
}

func (s *Service) handleDiskOps(ops []*pb.DiskOp, c *pushCounters) {
	for _, op := range ops {
		if !validateDiskOp(op) {
			c.rejectSample()
			continue
		}
		if !s.applyDiskOp(op) {
			c.rejectSample()
			continue
		}
		c.acceptSample()
	}
}

func validateDiskOp(op *pb.DiskOp) bool {
	return validID(op.GetDiskId()) && validLatency(op.GetLatencySeconds())
}

func (s *Service) applyDiskOp(op *pb.DiskOp) bool {
	diskID := op.GetDiskId()

	switch op.GetOp() {
	case pb.IoOpType_IO_OP_READ:
		recordIO(
			s.m.Disks.ReadOps.WithLabelValues(diskID),
			s.m.Disks.ReadBytes.WithLabelValues(diskID),
			s.m.Disks.ReadLatency.WithLabelValues(diskID),
			op.GetBytes(),
			op.GetLatencySeconds(),
		)
	case pb.IoOpType_IO_OP_WRITE:
		recordIO(
			s.m.Disks.WriteOps.WithLabelValues(diskID),
			s.m.Disks.WriteBytes.WithLabelValues(diskID),
			s.m.Disks.WriteLatency.WithLabelValues(diskID),
			op.GetBytes(),
			op.GetLatencySeconds(),
		)
	default:
		return false
	}

	if op.GetError() {
		s.m.Disks.Errors.WithLabelValues(diskID).Add(1)
	}

	return true
}

func (s *Service) handleDiskStates(states []*pb.DiskState, c *pushCounters) {
	for _, st := range states {
		if !validateDiskState(st) {
			c.rejectSample()
			continue
		}
		s.applyDiskState(st)
		c.acceptSample()
	}
}

func validateDiskState(st *pb.DiskState) bool {
	return validID(st.GetDiskId()) && finiteNonNeg(st.GetQueueDepth())
}

func (s *Service) applyDiskState(st *pb.DiskState) {
	s.m.Disks.QueueDepth.WithLabelValues(st.GetDiskId()).Set(st.GetQueueDepth())
}

func (s *Service) handleRaidOps(ops []*pb.RaidOp, c *pushCounters) {
	for _, op := range ops {
		if !validateRaidOp(op) {
			c.rejectSample()
			continue
		}
		if !s.applyRaidOp(op) {
			c.rejectSample()
			continue
		}
		c.acceptSample()
	}
}

func validateRaidOp(op *pb.RaidOp) bool {
	return validID(op.GetRaidId()) && validLatency(op.GetLatencySeconds())
}

func (s *Service) applyRaidOp(op *pb.RaidOp) bool {
	switch op.GetOp() {
	case pb.IoOpType_IO_OP_READ:
		s.applyRaidRead(op)
		return true
	case pb.IoOpType_IO_OP_WRITE:
		s.applyRaidWrite(op)
		return true
	default:
		return false
	}
}

func (s *Service) applyRaidRead(op *pb.RaidOp) {
	raidID := op.GetRaidId()

	recordIO(
		s.m.Raid.ReadOps.WithLabelValues(raidID),
		s.m.Raid.ReadBytes.WithLabelValues(raidID),
		s.m.Raid.ReadLatency.WithLabelValues(raidID),
		op.GetBytes(),
		op.GetLatencySeconds(),
	)

	if d := op.GetServedFromDiskId(); d != "" && validID(d) {
		s.m.Raid.Raid1ReadsFromDisk.WithLabelValues(raidID, d).Add(1)
	}

	if op.GetRaid3ParityRead() {
		s.m.Raid.Raid3ParityReads.WithLabelValues(raidID).Add(1)
	}
}

func (s *Service) applyRaidWrite(op *pb.RaidOp) {
	raidID := op.GetRaidId()

	recordIO(
		s.m.Raid.WriteOps.WithLabelValues(raidID),
		s.m.Raid.WriteBytes.WithLabelValues(raidID),
		s.m.Raid.WriteLatency.WithLabelValues(raidID),
		op.GetBytes(),
		op.GetLatencySeconds(),
	)

	if op.GetRaid3ParityWrite() {
		s.m.Raid.Raid3ParityWrites.WithLabelValues(raidID).Add(1)
	}

	if op.GetRaid3PartialStripeWrite() {
		s.m.Raid.Raid3PartialStripe.WithLabelValues(raidID).Add(1)
	}
}

func (s *Service) handleRaidStates(states []*pb.RaidState, c *pushCounters) {
	for _, st := range states {
		if !validateRaidState(st) {
			c.rejectSample()
			continue
		}
		s.applyRaidState(st)
		c.acceptSample()
	}
}

func validateRaidState(st *pb.RaidState) bool {
	return validID(st.GetRaidId()) && finiteNonNeg(st.GetRaid1ResyncProgress())
}

func (s *Service) applyRaidState(st *pb.RaidState) {
	raidID := st.GetRaidId()

	s.m.Raid.Raid1Resync.WithLabelValues(raidID).Set(st.GetRaid1ResyncProgress())
	setGaugeBool(s.m.Raid.DegradedState.WithLabelValues(raidID), st.GetDegraded())
	s.m.Raid.FailedDisks.WithLabelValues(raidID).Set(float64(st.GetFailedDisks()))
	setGaugeBool(s.m.Raid.RebuildInProgress.WithLabelValues(raidID), st.GetRebuildInProgress())
}

func (s *Service) handleFuseOps(ops []*pb.FuseOp, c *pushCounters) {
	for _, op := range ops {
		if !validateFuseOp(op) {
			c.rejectSample()
			continue
		}
		if !s.applyFuseOp(op) {
			c.rejectSample()
			continue
		}
		c.acceptSample()
	}
}

func validateFuseOp(op *pb.FuseOp) bool {
	return validLatency(op.GetLatencySeconds())
}

func (s *Service) applyFuseOp(op *pb.FuseOp) bool {
	switch op.GetOp() {
	case pb.FuseOpType_FUSE_OP_READ:
		recordIO(
			s.m.Fuse.ReadOps,
			s.m.Fuse.ReadBytes,
			s.m.Fuse.ReadLatency,
			op.GetBytes(),
			op.GetLatencySeconds(),
		)
	case pb.FuseOpType_FUSE_OP_WRITE:
		recordIO(
			s.m.Fuse.WriteOps,
			s.m.Fuse.WriteBytes,
			s.m.Fuse.WriteLatency,
			op.GetBytes(),
			op.GetLatencySeconds(),
		)
	case pb.FuseOpType_FUSE_OP_OPEN:
		s.m.Fuse.OpenOps.Add(1)
	case pb.FuseOpType_FUSE_OP_FSYNC:
		s.m.Fuse.FsyncOps.Add(1)
	default:
		return false
	}

	if op.GetError() {
		s.m.Fuse.Errors.Add(1)
	}

	return true
}

func (s *Service) handleProcess(ps *pb.ProcessSample, c *pushCounters) {
	if ps == nil {
		return
	}

	if finiteNonNeg(ps.GetCpuSeconds()) {
		s.m.Process.CPUSeconds.Set(ps.GetCpuSeconds())
		c.acceptSample()
	} else {
		c.rejectSample()
	}

	s.m.Process.ResidentMemory.Set(float64(ps.GetResidentMemoryBytes()))
	c.acceptSample()
}

func recordIO(ops prometheus.Counter, bytes prometheus.Counter, latency prometheus.Observer, nbytes uint64, latSec float64) {
	ops.Add(1)
	bytes.Add(float64(nbytes))
	observeIfPositive(latency, latSec)
}

func observeIfPositive(o prometheus.Observer, v float64) {
	if v > 0 {
		o.Observe(v)
	}
}

func setGaugeBool(g prometheus.Gauge, v bool) {
	if v {
		g.Set(1)
		return
	}
	g.Set(0)
}

func validID(s string) bool {
	return idRe.MatchString(s)
}

func validLatency(v float64) bool {
	return finiteNonNeg(v)
}

func finiteNonNeg(v float64) bool {
	return !math.IsNaN(v) && !math.IsInf(v, 0) && v >= 0
}
