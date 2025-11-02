#!/usr/bin/env bash
set -euo pipefail

mkdir -p target
tar xzf cov.tar.gz -C target

export CARGO_TARGET_DIR=target/llvm-cov-target

mkdir -p reports

cargo llvm-cov report \
  --cobertura --output-path reports/coverage.xml \
  --ignore-filename-regex='/.cargo/|/rustc/'

rate=$(grep -o 'line-rate="[0-9.]*"' reports/coverage.xml | head -1 | cut -d'"' -f2 || true)
if [ -z "${rate:-}" ]; then
  rate=0
fi

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || { [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] && [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ]; }; then
  awk -v r="$rate" 'BEGIN {
    if (r < 0.5) { printf "Coverage %.2f < 0.50. FAIL\n", r; exit 1 }
    else { printf "Coverage %.2f >= 0.50. OK\n", r }
  }'
else
  echo "Soft coverage check (branch): $rate"
fi
