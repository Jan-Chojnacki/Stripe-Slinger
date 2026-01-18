package server

import (
	"io"
	"metrics-gateway/internal/metrics"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestMetricsEndpointServesPrometheusOutput(t *testing.T) {
	reg, all := metrics.NewMetricsRegistry()

	all.Disks.ReadOps.WithLabelValues("disk0").Inc()

	mux := NewMux(reg)
	ts := httptest.NewServer(mux)
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/metrics")
	if err != nil {
		t.Fatalf("GET /metrics failed: %v", err)
	}
	defer func() {
		if cerr := resp.Body.Close(); cerr != nil {
			t.Logf("closing /metrics response body failed: %v", cerr)
		}
	}()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from /metrics, got %d", resp.StatusCode)
	}

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("reading /metrics body failed: %v", err)
	}
	body := string(bodyBytes)

	if !strings.Contains(body, "disk_read_ops") {
		t.Fatalf("expected metrics output to contain disk_read_ops, got:\n%s", body)
	}
}

func TestHealthzEndpointReturnsOK(t *testing.T) {
	reg, _ := metrics.NewMetricsRegistry()

	mux := NewMux(reg)
	ts := httptest.NewServer(mux)
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/healthz")
	if err != nil {
		t.Fatalf("GET /healthz failed: %v", err)
	}
	defer func() {
		if cerr := resp.Body.Close(); cerr != nil {
			t.Logf("closing /healthz response body failed: %v", cerr)
		}
	}()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from /healthz, got %d", resp.StatusCode)
	}

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("reading /healthz body failed: %v", err)
	}
	body := string(bodyBytes)

	if strings.TrimSpace(body) != "ok" {
		t.Fatalf(`expected body "ok", got %q`, body)
	}
}
