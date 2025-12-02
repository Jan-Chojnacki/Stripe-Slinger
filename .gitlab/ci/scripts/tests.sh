#!/bin/sh
set -euo pipefail

rm -rf target/llvm-cov-target target/llvm-cov-target-* target/nextest
rm -f cov.tar.gz cov-*.tar.gz

export CARGO_TARGET_DIR=target/llvm-cov-target

cargo llvm-cov nextest --workspace --all-features --no-report

tar czf cov.tar.gz -C target llvm-cov-target
