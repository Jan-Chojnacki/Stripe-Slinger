#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-lint] Running cargo clippy for $RUST_PROJECT_DIR..."

cargo clippy \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --workspace --all-targets --all-features \
  -- -D clippy::correctness -D clippy::suspicious -A dead_code -A unused_variables -A unused_mut

echo "[raid-simulator-lint] cargo clippy OK"
