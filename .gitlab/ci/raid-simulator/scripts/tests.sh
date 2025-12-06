#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-tests] Preparing coverage target directories..."

rm -rf target/llvm-cov-target target/llvm-cov-target-* target/nextest
rm -f cov.tar.gz cov-*.tar.gz

export CARGO_TARGET_DIR=target/llvm-cov-target

echo "[raid-simulator-tests] Running cargo llvm-cov nextest for $RUST_PROJECT_DIR..."
cargo llvm-cov nextest \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --workspace --all-features --no-report

echo "[raid-simulator-tests] Packaging coverage data into cov.tar.gz..."
tar czf cov.tar.gz -C target llvm-cov-target

echo "[raid-simulator-tests] Tests and coverage collection finished (cov.tar.gz created)"
