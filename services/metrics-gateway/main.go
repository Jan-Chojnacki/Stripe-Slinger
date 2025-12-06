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

	"github.com/prometheus/client_golang/prometheus/promhttp"
)

func main() {
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	reg, allMetrics := NewMetricsRegistry()

	diskIDs := []string{"disk0", "disk1", "disk2", "disk3"}
	raidIDs := []string{"raid0", "raid1", "raid3"}

	simulator := NewSimulator(allMetrics, diskIDs, raidIDs)

	var wg sync.WaitGroup
	simulator.Start(ctx, &wg, 1*time.Second)

	mux := http.NewServeMux()

	mux.Handle("/metrics", promhttp.HandlerFor(reg, promhttp.HandlerOpts{}))

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ok\n"))
	})

	addr := ":8080"
	if port := os.Getenv("METRICS_PORT"); port != "" {
		addr = ":" + port
	}

	server := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	go func() {
		log.Printf("Starting metrics server on %s", addr)

		if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("HTTP server error: %v", err)
		}
	}()

	<-ctx.Done()
	log.Println("Shutting down...")

	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer shutdownCancel()

	if err := server.Shutdown(shutdownCtx); err != nil {
		log.Printf("HTTP server shutdown error: %v", err)
	}

	wg.Wait()
	log.Println("Shutdown complete")
}
