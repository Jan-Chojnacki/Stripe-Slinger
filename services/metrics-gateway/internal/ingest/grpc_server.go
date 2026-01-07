package ingest

import (
	"context"
	"net"
	"os"

	"google.golang.org/grpc"
)

type Server struct {
	sockPath string
	lis      net.Listener
	grpcSrv  *grpc.Server
}

type Options struct {
	SocketMode os.FileMode
}

func (s *Server) Serve() error {
	return s.grpcSrv.Serve(s.lis)
}

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
