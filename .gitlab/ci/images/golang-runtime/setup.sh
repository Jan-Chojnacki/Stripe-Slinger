#!/bin/sh
set -eu

echo "[golang-runtime-setup] Starting Go CI runtime setup..."

echo "[golang-runtime-setup] Installing govulncheck..."
GO111MODULE=on go install golang.org/x/vuln/cmd/govulncheck@latest

if command -v govulncheck >/dev/null 2>&1; then
  echo "[golang-runtime-setup] govulncheck installed successfully."
else
  echo "[golang-runtime-setup] ERROR: govulncheck not found in PATH after installation."
  exit 1
fi

echo "[golang-runtime-setup] Go CI runtime setup completed."
