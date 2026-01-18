package ingest

import (
	"context"
	"net"
	"os"

	"google.golang.org/grpc"
)

// Server wraps a gRPC server bound to a Unix domain socket for metrics ingestion.
type Server struct {
	sockPath string
	lis      net.Listener
	grpcSrv  *grpc.Server
}

// Options configures socket settings for a Server.
type Options struct {
	SocketMode os.FileMode
}

// Serve starts serving gRPC requests on the configured listener.
func (s *Server) Serve() error {
	return s.grpcSrv.Serve(s.lis)
}

// Shutdown gracefully stops the gRPC server and removes the socket file.
func (s *Server) Shutdown(ctx context.Context) error {
	done := make(chan struct{})
	go func() {
		s.grpcSrv.GracefulStop()
		close(done)
	}()

	select {
	case <-done:
	case <-ctx.Done():
		s.grpcSrv.Stop()
	}

	_ = s.lis.Close()
	_ = os.Remove(s.sockPath)
	return nil
}
