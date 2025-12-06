#!/bin/sh
set -eu

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-tests] Starting Go tests with coverage..."
echo "[metrics-gateway-tests] Running go test ./... -coverprofile=coverage.out -covermode=atomic..."

go test ./... -coverprofile=coverage.out -covermode=atomic

echo "[metrics-gateway-tests] Tests OK, coverage.out created; tests job completed."
