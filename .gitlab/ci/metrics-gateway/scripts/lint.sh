#!/bin/sh
set -eu

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-lint] Starting go vet lint job..."
echo "[metrics-gateway-lint] Running go vet..."

go vet ./...

echo "[metrics-gateway-lint] go vet OK; lint job completed."
