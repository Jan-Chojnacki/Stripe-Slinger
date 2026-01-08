package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"metrics-gateway/internal/ingest"
	"metrics-gateway/internal/metrics"
	"metrics-gateway/internal/server"
	"metrics-gateway/internal/simulator"
)

func main() {
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	reg, allMetrics := metrics.NewMetricsRegistry()
	httpSrv := server.NewHTTPServer(httpAddrFromEnv(), reg)

	ingestSvc := ingest.NewService(allMetrics)

	grpcCfg, err := server.LoadGRPCConfigFromEnv()
	if err != nil {
		log.Fatalf("load gRPC config: %v", err)
	}

	udsSrv, err := server.NewGRPCUDSServer(grpcCfg, ingestSvc)
	if err != nil {
		log.Fatalf("create gRPC UDS server: %v", err)
	}

	var wg sync.WaitGroup
	maybeStartSimulator(ctx, &wg, allMetrics)

	startHTTP(httpSrv)
	startGRPC(udsSrv, grpcCfg)

	<-ctx.Done()
	log.Println("Shutting down...")

	shutdownAll(&wg, httpSrv, udsSrv)

	log.Println("Shutdown complete")
}

func httpAddrFromEnv() string {
	if port := os.Getenv("METRICS_PORT"); port != "" {
		return ":" + port
	}
	return ":8080"
}

func maybeStartSimulator(ctx context.Context, wg *sync.WaitGroup, allMetrics *metrics.AllMetrics) {
	if !parseBool(getenvDefault("METRICS_ENABLE_SIMULATOR", "false")) {
		return
	}

	diskIDs := []string{"disk0", "disk1", "disk2", "disk3"}
	raidIDs := []string{"raid0", "raid1", "raid3"}

	sim := simulator.NewSimulator(allMetrics, diskIDs, raidIDs)
	sim.Start(ctx, wg, 1*time.Second)

	log.Printf("Go simulator enabled (METRICS_ENABLE_SIMULATOR=true)")
}

func startHTTP(httpSrv *http.Server) {
	go func() {
		log.Printf("Starting metrics HTTP server on %s", httpSrv.Addr)
		if err := httpSrv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("HTTP server error: %v", err)
		}
	}()
}

func startGRPC(udsSrv *server.GRPCUDSServer, cfg server.GRPCConfig) {
	go func() {
		logGRPCConfig(cfg)

		if err := udsSrv.Serve(); err != nil {
			log.Fatalf("gRPC UDS server error: %v", err)
		}
	}()
}

func logGRPCConfig(cfg server.GRPCConfig) {
	log.Printf("Starting gRPC UDS server on %s (mode=%#o)", cfg.UDSPath, cfg.SocketMode)

	if cfg.AuthToken != "" {
		log.Printf("gRPC auth: enabled")
	} else {
		log.Printf("gRPC auth: disabled")
	}

	if cfg.RateLimitRPS > 0 && cfg.RateLimitBurst > 0 {
		log.Printf("gRPC rate limit: rps=%.2f burst=%d", cfg.RateLimitRPS, cfg.RateLimitBurst)
	}
}

func shutdownAll(wg *sync.WaitGroup, httpSrv *http.Server, udsSrv *server.GRPCUDSServer) {
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := httpSrv.Shutdown(shutdownCtx); err != nil {
		log.Printf("HTTP server shutdown error: %v", err)
	}

	if err := udsSrv.Shutdown(shutdownCtx); err != nil {
		log.Printf("gRPC UDS shutdown error: %v", err)
	}

	wg.Wait()
}

func getenvDefault(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

func parseBool(v string) bool {
	switch v {
	case "1", "true", "TRUE", "True", "yes", "YES", "y", "Y":
		return true
	default:
		return false
	}
}
