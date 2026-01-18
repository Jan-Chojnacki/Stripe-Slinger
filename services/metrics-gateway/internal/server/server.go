package server

import (
	"net/http"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// NewMux builds an HTTP mux exposing metrics and a health check endpoint.
func NewMux(reg *prometheus.Registry) *http.ServeMux {
	mux := http.NewServeMux()

	mux.Handle("/metrics", promhttp.HandlerFor(reg, promhttp.HandlerOpts{}))

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ok\n"))
	})

	return mux
}

// NewHTTPServer constructs an HTTP server bound to addr with metrics handlers registered.
func NewHTTPServer(addr string, reg *prometheus.Registry) *http.Server {
	mux := NewMux(reg)

	return &http.Server{
		Addr:    addr,
		Handler: mux,
	}
}
