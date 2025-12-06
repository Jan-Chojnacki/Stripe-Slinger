package main

import (
	"context"
	"math/rand"
	"sync"
	"time"
)

type Simulator struct {
	metrics *AllMetrics
	diskIDs []string
	raidIDs []string

	rnd         *rand.Rand
	cpuSeconds  float64
	memoryBytes float64
}

func NewSimulator(metrics *AllMetrics, diskIDs, raidIDs []string) *Simulator {
	src := rand.NewSource(time.Now().UnixNano())
	rnd := rand.New(src)

	s := &Simulator{
		metrics: metrics,
		diskIDs: diskIDs,
		raidIDs: raidIDs,
		rnd:     rnd,
	}

	s.cpuSeconds = 0
	s.memoryBytes = 200*1024*1024 + float64(rnd.Intn(200*1024*1024))

	return s
}

func (s *Simulator) Start(ctx context.Context, wg *sync.WaitGroup, interval time.Duration) {
	wg.Add(1)

	go func() {
		defer wg.Done()

		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				s.tick()
			}
		}
	}()
}

func (s *Simulator) tick() {
	s.simulateDisks()
	s.simulateRaid()
	s.simulateFuse()
	s.simulateProcess()
}

func (s *Simulator) simulateDisks() {
	for _, diskID := range s.diskIDs {
		reads := float64(s.rnd.Intn(100))
		writes := float64(s.rnd.Intn(100))

		s.metrics.Disks.ReadOps.WithLabelValues(diskID).Add(reads)
		s.metrics.Disks.WriteOps.WithLabelValues(diskID).Add(writes)

		readBytesPerOp := float64(4096 + s.rnd.Intn(64*1024-4096))
		writeBytesPerOp := float64(4096 + s.rnd.Intn(64*1024-4096))

		s.metrics.Disks.ReadBytes.WithLabelValues(diskID).Add(reads * readBytesPerOp)
		s.metrics.Disks.WriteBytes.WithLabelValues(diskID).Add(writes * writeBytesPerOp)

		readLatency := 0.0001 + s.rnd.Float64()*0.005
		writeLatency := 0.0001 + s.rnd.Float64()*0.005

		s.metrics.Disks.ReadLatency.WithLabelValues(diskID).Set(readLatency)
		s.metrics.Disks.WriteLatency.WithLabelValues(diskID).Set(writeLatency)

		queueDepth := float64(s.rnd.Intn(32))
		s.metrics.Disks.QueueDepth.WithLabelValues(diskID).Set(queueDepth)

		if s.rnd.Float64() < 0.01 {
			s.metrics.Disks.Errors.WithLabelValues(diskID).Inc()
		}
	}
}

func (s *Simulator) simulateRaid() {
	m := s.metrics.Raid
	for _, raid := range s.raidIDs {
		s.simulateRaidVolume(m, raid)
	}
}

func (s *Simulator) simulateRaidVolume(m *RaidMetrics, raid string) {
	reads := float64(s.rnd.Intn(400))
	writes := float64(s.rnd.Intn(400))

	s.updateRaidIO(m, raid, reads, writes)
	s.updateRaidLatency(m, raid)
	s.updateRaidHealth(m, raid)
	s.updateRaidSpecificMetrics(m, raid)
}

func (s *Simulator) updateRaidIO(m *RaidMetrics, raid string, reads, writes float64) {
	m.ReadOps.WithLabelValues(raid).Add(reads)
	m.WriteOps.WithLabelValues(raid).Add(writes)

	readBytesPerOp := float64(16*1024 + s.rnd.Intn(128*1024))
	writeBytesPerOp := float64(16*1024 + s.rnd.Intn(128*1024))

	m.ReadBytes.WithLabelValues(raid).Add(reads * readBytesPerOp)
	m.WriteBytes.WithLabelValues(raid).Add(writes * writeBytesPerOp)
}

func (s *Simulator) updateRaidLatency(m *RaidMetrics, raid string) {
	readLatency := 0.0002 + s.rnd.Float64()*0.004
	writeLatency := 0.0002 + s.rnd.Float64()*0.004

	m.ReadLatency.WithLabelValues(raid).Set(readLatency)
	m.WriteLatency.WithLabelValues(raid).Set(writeLatency)
}

func (s *Simulator) updateRaidHealth(m *RaidMetrics, raid string) {
	degraded := s.rnd.Float64() < 0.05
	if !degraded {
		s.resetRaidHealth(m, raid)
		return
	}

	m.DegradedState.WithLabelValues(raid).Set(1)

	failed := float64(1 + s.rnd.Intn(2))
	m.FailedDisks.WithLabelValues(raid).Set(failed)

	rebuild := s.simulateRaidRebuild(m, raid)
	m.RebuildInProgress.WithLabelValues(raid).Set(rebuild)
}

func (s *Simulator) resetRaidHealth(m *RaidMetrics, raid string) {
	m.DegradedState.WithLabelValues(raid).Set(0)
	m.FailedDisks.WithLabelValues(raid).Set(0)
	m.RebuildInProgress.WithLabelValues(raid).Set(0)

	if raid == "raid1" {
		m.Raid1Resync.WithLabelValues(raid).Set(0)
	}
}

func (s *Simulator) simulateRaidRebuild(m *RaidMetrics, raid string) float64 {
	if s.rnd.Float64() >= 0.7 {
		if raid == "raid1" {
			m.Raid1Resync.WithLabelValues(raid).Set(0)
		}
		return 0
	}

	if raid == "raid1" {
		progress := s.rnd.Float64()
		m.Raid1Resync.WithLabelValues(raid).Set(progress)
	}
	return 1
}

func (s *Simulator) updateRaidSpecificMetrics(m *RaidMetrics, raid string) {
	switch raid {
	case "raid1":
		for _, diskID := range s.diskIDs {
			m.Raid1ReadsFromDisk.WithLabelValues(raid, diskID).
				Add(float64(s.rnd.Intn(200)))
		}
	case "raid3":
		m.Raid3ParityReads.WithLabelValues(raid).Add(float64(s.rnd.Intn(200)))
		m.Raid3ParityWrites.WithLabelValues(raid).Add(float64(s.rnd.Intn(200)))
		m.Raid3PartialStripe.WithLabelValues(raid).Add(float64(s.rnd.Intn(50)))
	}
}

func (s *Simulator) simulateFuse() {
	m := s.metrics.Fuse

	reads := float64(s.rnd.Intn(500))
	writes := float64(s.rnd.Intn(500))
	opens := float64(s.rnd.Intn(200))
	fsyncs := float64(s.rnd.Intn(100))

	m.ReadOps.Add(reads)
	m.WriteOps.Add(writes)
	m.OpenOps.Add(opens)
	m.FsyncOps.Add(fsyncs)

	readBytes := reads * float64(4096+s.rnd.Intn(64*1024))
	writeBytes := writes * float64(4096+s.rnd.Intn(64*1024))

	m.ReadBytes.Add(readBytes)
	m.WriteBytes.Add(writeBytes)

	readLatency := 0.0002 + s.rnd.Float64()*0.003
	writeLatency := 0.0002 + s.rnd.Float64()*0.003

	m.ReadLatency.Set(readLatency)
	m.WriteLatency.Set(writeLatency)

	if s.rnd.Float64() < 0.02 {
		m.Errors.Inc()
	}
}

func (s *Simulator) simulateProcess() {
	cpuDelta := 0.01 + s.rnd.Float64()*0.2
	s.cpuSeconds += cpuDelta
	s.metrics.Process.CPUSeconds.Set(s.cpuSeconds)

	drift := float64(s.rnd.Intn(5 * 1024 * 1024))
	if s.rnd.Intn(2) == 0 {
		s.memoryBytes += drift
	} else {
		s.memoryBytes -= drift
	}

	if s.memoryBytes < 50*1024*1024 {
		s.memoryBytes = 50 * 1024 * 1024
	}

	s.metrics.Process.ResidentMemory.Set(s.memoryBytes)
}
