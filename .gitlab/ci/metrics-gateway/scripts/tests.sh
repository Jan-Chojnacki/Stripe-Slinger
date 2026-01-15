#!/bin/sh
set -eu

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-tests] Starting Go tests with coverage..."
echo "[metrics-gateway-tests] Running go test ./... -coverprofile=coverage.out.tmp -covermode=atomic..."

go test ./... -coverprofile=coverage.out.tmp -covermode=atomic

echo "[metrics-gateway-tests] Filtering out generated files from /pb/ directory..."

head -n 1 coverage.out.tmp > coverage.out
grep -v "/pb/" coverage.out.tmp | grep -v "mode: " >> coverage.out

rm coverage.out.tmp

echo "[metrics-gateway-tests] Tests OK, filtered coverage.out created; tests job completed."
