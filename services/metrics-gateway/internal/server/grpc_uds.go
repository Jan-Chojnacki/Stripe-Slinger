package server

import (
	"context"
	"errors"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	pb "metrics-gateway/internal/pb/metrics/v1"

	"golang.org/x/time/rate"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/keepalive"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

type GRPCConfig struct {
	UDSPath    string
	SocketMode os.FileMode

	MaxRecvBytes         int
	MaxConcurrentStreams uint32
	AuthToken            string
	RateLimitRPS         float64
	RateLimitBurst       int
	KAEnforcementMinTime time.Duration
	KATime               time.Duration
	KATimeout            time.Duration
	MaxConnIdle          time.Duration
	MaxConnAge           time.Duration
	MaxConnAgeGrace      time.Duration
}

func LoadGRPCConfigFromEnv() (GRPCConfig, error) {
	cfg := GRPCConfig{
		UDSPath:              getenv("GRPC_UDS_PATH", "/sockets/metrics-gateway.sock"),
		SocketMode:           parseFileMode(getenv("GRPC_UDS_SOCKET_MODE", "660"), 0660),
		MaxRecvBytes:         parseInt(getenv("GRPC_MAX_RECV_BYTES", "4194304"), 4<<20),
		MaxConcurrentStreams: uint32(parseInt(getenv("GRPC_MAX_CONCURRENT_STREAMS", "1024"), 1024)),
		AuthToken:            os.Getenv("GRPC_AUTH_TOKEN"),
		RateLimitRPS:         parseFloat(getenv("GRPC_RATELIMIT_RPS", "0"), 0),
		RateLimitBurst:       parseInt(getenv("GRPC_RATELIMIT_BURST", "0"), 0),
		KAEnforcementMinTime: parseDurationMS(getenv("GRPC_KA_MIN_TIME_MS", "30000"), 30*time.Second),
		KATime:               parseDurationMS(getenv("GRPC_KA_TIME_MS", "120000"), 2*time.Minute),
		KATimeout:            parseDurationMS(getenv("GRPC_KA_TIMEOUT_MS", "20000"), 20*time.Second),
		MaxConnIdle:          parseDurationMS(getenv("GRPC_MAX_CONN_IDLE_MS", "300000"), 5*time.Minute),
		MaxConnAge:           parseDurationMS(getenv("GRPC_MAX_CONN_AGE_MS", "1800000"), 30*time.Minute),
		MaxConnAgeGrace:      parseDurationMS(getenv("GRPC_MAX_CONN_AGE_GRACE_MS", "60000"), 1*time.Minute),
	}

	if cfg.UDSPath == "" {
		return GRPCConfig{}, fmt.Errorf("GRPC_UDS_PATH is empty")
	}
	return cfg, nil
}

type GRPCUDSServer struct {
	cfg  GRPCConfig
	srv  *grpc.Server
	lis  net.Listener
	path string

	doneOnce sync.Once
	doneCh   chan struct{}
}

func NewGRPCUDSServer(cfg GRPCConfig, ingest pb.MetricsIngestorServer) (*GRPCUDSServer, error) {
	if err := ensureSocketDir(cfg.UDSPath); err != nil {
		return nil, err
	}

	_ = os.Remove(cfg.UDSPath)

	lis, err := net.Listen("unix", cfg.UDSPath)
	if err != nil {
		return nil, fmt.Errorf("listen unix %s: %w", cfg.UDSPath, err)
	}

	if err := os.Chmod(cfg.UDSPath, cfg.SocketMode); err != nil {
		_ = lis.Close()
		return nil, fmt.Errorf("chmod %s: %w", cfg.UDSPath, err)
	}

	var limiter *rate.Limiter
	if cfg.RateLimitRPS > 0 && cfg.RateLimitBurst > 0 {
		limiter = rate.NewLimiter(rate.Limit(cfg.RateLimitRPS), cfg.RateLimitBurst)
	}

	ua := unaryAuthInterceptor(cfg.AuthToken)
	sa := streamAuthInterceptor(cfg.AuthToken)

	ur := unaryRateInterceptor(limiter)
	sr := streamRateInterceptor(limiter)

	grpcSrv := grpc.NewServer(
		grpc.MaxRecvMsgSize(cfg.MaxRecvBytes),
		grpc.MaxConcurrentStreams(cfg.MaxConcurrentStreams),

		grpc.KeepaliveEnforcementPolicy(keepalive.EnforcementPolicy{
			MinTime:             cfg.KAEnforcementMinTime,
			PermitWithoutStream: true,
		}),
		grpc.KeepaliveParams(keepalive.ServerParameters{
			Time:                  cfg.KATime,
			Timeout:               cfg.KATimeout,
			MaxConnectionIdle:     cfg.MaxConnIdle,
			MaxConnectionAge:      cfg.MaxConnAge,
			MaxConnectionAgeGrace: cfg.MaxConnAgeGrace,
		}),

		grpc.ChainUnaryInterceptor(ua, ur),
		grpc.ChainStreamInterceptor(sa, sr),
	)

	pb.RegisterMetricsIngestorServer(grpcSrv, ingest)

	return &GRPCUDSServer{
		cfg:    cfg,
		srv:    grpcSrv,
		lis:    lis,
		path:   cfg.UDSPath,
		doneCh: make(chan struct{}),
	}, nil
}

func (s *GRPCUDSServer) Serve() error {
	defer s.markDone()

	err := s.srv.Serve(s.lis)
	if err != nil && !errors.Is(err, net.ErrClosed) {
		return err
	}
	return nil
}

func (s *GRPCUDSServer) Shutdown(ctx context.Context) error {
	stopped := make(chan struct{})
	go func() {
		defer close(stopped)
		s.srv.GracefulStop()
	}()

	select {
	case <-stopped:
	case <-ctx.Done():
		s.srv.Stop()
	}

	_ = s.lis.Close()
	_ = os.Remove(s.path)

	<-s.doneCh
	return nil
}

func (s *GRPCUDSServer) markDone() {
	s.doneOnce.Do(func() {
		close(s.doneCh)
	})
}

func ensureSocketDir(sockPath string) error {
	dir := filepath.Dir(sockPath)
	if dir == "." || dir == "/" {
		return nil
	}
	if err := os.MkdirAll(dir, 0770); err != nil {
		return fmt.Errorf("mkdir %s: %w", dir, err)
	}
	return nil
}

func unaryAuthInterceptor(token string) grpc.UnaryServerInterceptor {
	token = strings.TrimSpace(token)
	if token == "" {
		return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
			return handler(ctx, req)
		}
	}

	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		if !checkAuth(ctx, token) {
			return nil, status.Error(codes.Unauthenticated, "unauthorized")
		}
		return handler(ctx, req)
	}
}

func streamAuthInterceptor(token string) grpc.StreamServerInterceptor {
	token = strings.TrimSpace(token)
	if token == "" {
		return func(srv any, ss grpc.ServerStream, info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
			return handler(srv, ss)
		}
	}

	return func(srv any, ss grpc.ServerStream, info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
		if !checkAuth(ss.Context(), token) {
			return status.Error(codes.Unauthenticated, "unauthorized")
		}
		return handler(srv, ss)
	}
}

func checkAuth(ctx context.Context, token string) bool {
	md, ok := metadata.FromIncomingContext(ctx)
	if !ok {
		return false
	}

	if vals := md.Get("x-metrics-token"); len(vals) > 0 && strings.TrimSpace(vals[0]) == token {
		return true
	}

	if vals := md.Get("authorization"); len(vals) > 0 {
		v := strings.TrimSpace(vals[0])
		if strings.HasPrefix(strings.ToLower(v), "bearer ") {
			v = strings.TrimSpace(v[7:])
		}
		return v == token
	}

	return false
}

func unaryRateInterceptor(l *rate.Limiter) grpc.UnaryServerInterceptor {
	if l == nil {
		return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
			return handler(ctx, req)
		}
	}

	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		if err := l.Wait(ctx); err != nil {
			return nil, status.Error(codes.ResourceExhausted, "rate limited")
		}
		return handler(ctx, req)
	}
}

func streamRateInterceptor(l *rate.Limiter) grpc.StreamServerInterceptor {
	if l == nil {
		return func(srv any, ss grpc.ServerStream, info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
			return handler(srv, ss)
		}
	}

	return func(srv any, ss grpc.ServerStream, info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
		wrapped := &rateLimitedServerStream{ServerStream: ss, limiter: l}
		return handler(srv, wrapped)
	}
}

type rateLimitedServerStream struct {
	grpc.ServerStream
	limiter *rate.Limiter
}

func (s *rateLimitedServerStream) RecvMsg(m any) error {
	if err := s.limiter.Wait(s.Context()); err != nil {
		return status.Error(codes.ResourceExhausted, "rate limited")
	}
	return s.ServerStream.RecvMsg(m)
}

func getenv(k, def string) string {
	v := os.Getenv(k)
	if v == "" {
		return def
	}
	return v
}

func parseInt(s string, def int) int {
	i, err := strconv.Atoi(strings.TrimSpace(s))
	if err != nil {
		return def
	}
	return i
}

func parseFloat(s string, def float64) float64 {
	f, err := strconv.ParseFloat(strings.TrimSpace(s), 64)
	if err != nil {
		return def
	}
	return f
}

func parseDurationMS(s string, def time.Duration) time.Duration {
	ms, err := strconv.Atoi(strings.TrimSpace(s))
	if err != nil || ms < 0 {
		return def
	}
	return time.Duration(ms) * time.Millisecond
}

func parseFileMode(s string, def os.FileMode) os.FileMode {
	s = strings.TrimSpace(s)
	if s == "" {
		return def
	}
	v, err := strconv.ParseUint(s, 8, 32)
	if err != nil {
		return def
	}
	return os.FileMode(v)
}
