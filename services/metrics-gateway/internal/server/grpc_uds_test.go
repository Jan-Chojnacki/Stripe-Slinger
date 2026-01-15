package server

import (
	"context"
	"net"
	"path/filepath"
	"testing"
	"time"

	pb "metrics-gateway/internal/pb/metrics/v1"

	"golang.org/x/time/rate"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

type fakeIngestServer struct {
	pb.UnimplementedMetricsIngestorServer
}

type fakeServerStream struct {
	ctx context.Context
}

func (f *fakeServerStream) SetHeader(metadata.MD) error  { return nil }
func (f *fakeServerStream) SendHeader(metadata.MD) error { return nil }
func (f *fakeServerStream) SetTrailer(metadata.MD)       {}
func (f *fakeServerStream) Context() context.Context     { return f.ctx }
func (f *fakeServerStream) SendMsg(any) error            { return nil }
func (f *fakeServerStream) RecvMsg(any) error            { return nil }

func TestParseHelpers(t *testing.T) {
	if v := parseInt("not-a-number", 5); v != 5 {
		t.Fatalf("expected parseInt fallback, got %d", v)
	}
	if v := parseFloat("bad", 1.5); v != 1.5 {
		t.Fatalf("expected parseFloat fallback, got %f", v)
	}
	if v := parseDurationMS("-1", 2*time.Second); v != 2*time.Second {
		t.Fatalf("expected parseDurationMS fallback, got %s", v)
	}
	if v := parseFileMode("bad", 0640); v != 0640 {
		t.Fatalf("expected parseFileMode fallback, got %v", v)
	}
}

func TestAuthInterceptors(t *testing.T) {
	token := "secret"
	ctx := metadata.NewIncomingContext(context.Background(), metadata.Pairs("x-metrics-token", token))

	unary := unaryAuthInterceptor(token)
	if _, err := unary(ctx, nil, nil, func(ctx context.Context, req any) (any, error) {
		return "ok", nil
	}); err != nil {
		t.Fatalf("expected unary auth to pass: %v", err)
	}

	stream := streamAuthInterceptor(token)
	ss := &fakeServerStream{ctx: ctx}
	if err := stream(nil, ss, nil, func(srv any, stream grpc.ServerStream) error { return nil }); err != nil {
		t.Fatalf("expected stream auth to pass: %v", err)
	}

	denyCtx := metadata.NewIncomingContext(context.Background(), metadata.Pairs("x-metrics-token", "bad"))
	if _, err := unary(denyCtx, nil, nil, func(ctx context.Context, req any) (any, error) {
		return nil, nil
	}); status.Code(err) != codes.Unauthenticated {
		t.Fatalf("expected unauthenticated, got %v", err)
	}
}

func TestRateInterceptors(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	limiter := rate.NewLimiter(rate.Limit(1), 1)
	unary := unaryRateInterceptor(limiter)
	if _, err := unary(ctx, nil, nil, func(ctx context.Context, req any) (any, error) {
		return nil, nil
	}); status.Code(err) != codes.ResourceExhausted {
		t.Fatalf("expected rate limited error, got %v", err)
	}

	stream := streamRateInterceptor(limiter)
	ss := &rateLimitedServerStream{
		ServerStream: &fakeServerStream{ctx: ctx},
		limiter:      limiter,
	}
	if err := stream(nil, ss, nil, func(srv any, stream grpc.ServerStream) error {
		return stream.RecvMsg(nil)
	}); status.Code(err) != codes.ResourceExhausted {
		t.Fatalf("expected rate limited stream error, got %v", err)
	}
}

func TestLoadGRPCConfigFromEnv(t *testing.T) {
	t.Setenv("GRPC_UDS_PATH", filepath.Join(t.TempDir(), "srv.sock"))
	t.Setenv("GRPC_UDS_SOCKET_MODE", "600")
	t.Setenv("GRPC_MAX_RECV_BYTES", "2048")
	t.Setenv("GRPC_MAX_CONCURRENT_STREAMS", "5")
	t.Setenv("GRPC_AUTH_TOKEN", "tok")
	t.Setenv("GRPC_RATELIMIT_RPS", "2.5")
	t.Setenv("GRPC_RATELIMIT_BURST", "3")
	t.Setenv("GRPC_KA_MIN_TIME_MS", "10")
	t.Setenv("GRPC_KA_TIME_MS", "11")
	t.Setenv("GRPC_KA_TIMEOUT_MS", "12")
	t.Setenv("GRPC_MAX_CONN_IDLE_MS", "13")
	t.Setenv("GRPC_MAX_CONN_AGE_MS", "14")
	t.Setenv("GRPC_MAX_CONN_AGE_GRACE_MS", "15")

	cfg, err := LoadGRPCConfigFromEnv()
	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if cfg.MaxRecvBytes != 2048 || cfg.MaxConcurrentStreams != 5 {
		t.Fatalf("unexpected limits: %+v", cfg)
	}
	if cfg.AuthToken != "tok" || cfg.RateLimitRPS != 2.5 || cfg.RateLimitBurst != 3 {
		t.Fatalf("unexpected auth/limit: %+v", cfg)
	}
	if cfg.SocketMode != 0600 {
		t.Fatalf("unexpected socket mode: %v", cfg.SocketMode)
	}
	if cfg.KAEnforcementMinTime != 10*time.Millisecond || cfg.KATime != 11*time.Millisecond ||
		cfg.KATimeout != 12*time.Millisecond || cfg.MaxConnIdle != 13*time.Millisecond ||
		cfg.MaxConnAge != 14*time.Millisecond || cfg.MaxConnAgeGrace != 15*time.Millisecond {
		t.Fatalf("unexpected keepalive durations: %+v", cfg)
	}
}

func TestEnsureSocketDir(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "nested")
	path := filepath.Join(dir, "sock")
	if err := ensureSocketDir(path); err != nil {
		t.Fatalf("ensureSocketDir: %v", err)
	}
	if err := ensureSocketDir("/"); err != nil {
		t.Fatalf("ensureSocketDir for /: %v", err)
	}
	if err := ensureSocketDir("."); err != nil {
		t.Fatalf("ensureSocketDir for .: %v", err)
	}
}

func TestCheckAuthBearer(t *testing.T) {
	token := "bearer-token"
	ctx := metadata.NewIncomingContext(context.Background(), metadata.Pairs("authorization", "Bearer "+token))
	if !checkAuth(ctx, token) {
		t.Fatal("expected bearer token to be accepted")
	}
	if checkAuth(context.Background(), token) {
		t.Fatal("expected missing metadata to be rejected")
	}
}

func TestInterceptorsNoTokenOrLimiter(t *testing.T) {
	unary := unaryAuthInterceptor("")
	if _, err := unary(context.Background(), nil, nil, func(ctx context.Context, req any) (any, error) {
		return "ok", nil
	}); err != nil {
		t.Fatalf("expected no-token unary to pass: %v", err)
	}

	stream := streamAuthInterceptor("")
	if err := stream(nil, &fakeServerStream{ctx: context.Background()}, nil, func(srv any, stream grpc.ServerStream) error {
		return nil
	}); err != nil {
		t.Fatalf("expected no-token stream to pass: %v", err)
	}

	rateUnary := unaryRateInterceptor(nil)
	if _, err := rateUnary(context.Background(), nil, nil, func(ctx context.Context, req any) (any, error) {
		return "ok", nil
	}); err != nil {
		t.Fatalf("expected nil limiter unary to pass: %v", err)
	}

	rateStream := streamRateInterceptor(nil)
	if err := rateStream(nil, &fakeServerStream{ctx: context.Background()}, nil, func(srv any, stream grpc.ServerStream) error {
		return nil
	}); err != nil {
		t.Fatalf("expected nil limiter stream to pass: %v", err)
	}
}

func TestGRPCUDSServerLifecycle(t *testing.T) {
	dir := t.TempDir()
	cfg := GRPCConfig{
		UDSPath:              filepath.Join(dir, "metrics.sock"),
		SocketMode:           0600,
		MaxRecvBytes:         1024,
		MaxConcurrentStreams: 10,
		KAEnforcementMinTime: time.Millisecond,
		KATime:               time.Millisecond,
		KATimeout:            time.Millisecond,
		MaxConnIdle:          time.Millisecond,
		MaxConnAge:           time.Millisecond,
		MaxConnAgeGrace:      time.Millisecond,
	}

	srv, err := NewGRPCUDSServer(cfg, &fakeIngestServer{})
	if err != nil {
		t.Fatalf("new server: %v", err)
	}

	done := make(chan error, 1)
	go func() {
		done <- srv.Serve()
	}()

	if conn, err := net.Dial("unix", cfg.UDSPath); err == nil {
		_ = conn.Close()
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		t.Fatalf("shutdown: %v", err)
	}

	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("server did not stop")
	}

	if _, err := net.Dial("unix", cfg.UDSPath); err == nil {
		t.Fatal("expected socket to be removed")
	}
}
