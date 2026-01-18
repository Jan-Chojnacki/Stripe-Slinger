package ingest

import (
	"context"
	"net"
	"path/filepath"
	"testing"
	"time"

	"google.golang.org/grpc"
)

func TestServerShutdownRemovesSocket(t *testing.T) {
	dir := t.TempDir()
	sock := filepath.Join(dir, "ingest.sock")

	lis, err := net.Listen("unix", sock)
	if err != nil {
		t.Fatalf("listen: %v", err)
	}

	s := &Server{
		sockPath: sock,
		lis:      lis,
		grpcSrv:  grpc.NewServer(),
	}

	done := make(chan error, 1)
	go func() {
		done <- s.Serve()
	}()

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	if err := s.Shutdown(ctx); err != nil {
		t.Fatalf("shutdown: %v", err)
	}

	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("server did not stop")
	}

	if _, err := net.Dial("unix", sock); err == nil {
		t.Fatal("expected socket to be removed")
	}
}
