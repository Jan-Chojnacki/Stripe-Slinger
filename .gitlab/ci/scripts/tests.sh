#!/usr/bin/env bash
set -euo pipefail

export CARGO_TARGET_DIR=target/llvm-cov-target

cargo llvm-cov nextest --workspace --all-features --no-report

tar czf cov.tar.gz -C target llvm-cov-target
