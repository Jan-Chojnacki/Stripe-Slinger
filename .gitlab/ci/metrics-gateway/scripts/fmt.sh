#!/bin/sh
set -euo pipefail

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-fmt] Checking gofmt formatting..."

fmt_output="$(gofmt -l . || true)"

if [ -n "$fmt_output" ]; then
  echo "[metrics-gateway-fmt] The following files are not gofmt-formatted:"
  echo "$fmt_output"
  exit 1
fi

echo "[metrics-gateway-fmt] gofmt OK"
