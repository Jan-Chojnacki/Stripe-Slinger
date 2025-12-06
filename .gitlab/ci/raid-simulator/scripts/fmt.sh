#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-fmt] Starting rustfmt check..."
echo "[raid-simulator-fmt] Checking rustfmt formatting for $RUST_PROJECT_DIR..."

cargo fmt \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --all -- --check

echo "[raid-simulator-fmt] rustfmt OK; fmt job completed."
