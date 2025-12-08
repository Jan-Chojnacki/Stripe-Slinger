package server

import (
	"net/http"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

func NewMux(reg *prometheus.Registry) *http.ServeMux {
	mux := http.NewServeMux()

	mux.Handle("/metrics", promhttp.HandlerFor(reg, promhttp.HandlerOpts{}))

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ok\n"))
	})

	return mux
}

func NewHTTPServer(addr string, reg *prometheus.Registry) *http.Server {
	mux := NewMux(reg)

	return &http.Server{
		Addr:    addr,
		Handler: mux,
	}
}
