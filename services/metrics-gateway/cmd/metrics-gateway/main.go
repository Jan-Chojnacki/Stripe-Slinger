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

	"metrics-gateway/internal/metrics"
	"metrics-gateway/internal/server"
	"metrics-gateway/internal/simulator"
)

func main() {
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	reg, allMetrics := metrics.NewMetricsRegistry()

	diskIDs := []string{"disk0", "disk1", "disk2", "disk3"}
	raidIDs := []string{"raid0", "raid1", "raid3"}

	sim := simulator.NewSimulator(allMetrics, diskIDs, raidIDs)

	var wg sync.WaitGroup
	sim.Start(ctx, &wg, 1*time.Second)

	addr := ":8080"
	if port := os.Getenv("METRICS_PORT"); port != "" {
		addr = ":" + port
	}

	srv := server.NewHTTPServer(addr, reg)

	go func() {
		log.Printf("Starting metrics server on %s", addr)

		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("HTTP server error: %v", err)
		}
	}()

	<-ctx.Done()
	log.Println("Shutting down...")

	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer shutdownCancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Printf("HTTP server shutdown error: %v", err)
	}

	wg.Wait()
	log.Println("Shutdown complete")
}
