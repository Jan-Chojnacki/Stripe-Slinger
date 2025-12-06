#!/bin/sh
set -euo pipefail

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-tests] Running go test with coverage..."

go test ./... -coverprofile=coverage.out -covermode=atomic

echo "[metrics-gateway-tests] Tests OK, coverage.out created"
