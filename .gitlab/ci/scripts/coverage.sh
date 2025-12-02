#!/bin/sh
set -euo pipefail

mkdir -p target
tar xzf cov.tar.gz -C target

export CARGO_TARGET_DIR=target/llvm-cov-target
export LLVM_COV_FLAGS="${LLVM_COV_FLAGS:-} -use-color=0"

mkdir -p reports

report_out=$(cargo llvm-cov report \
  --ignore-filename-regex='/.cargo/|/rustc/')

rate=$(
  printf '%s\n' "$report_out" \
  | awk '/^TOTAL/ { gsub("%","",$10); if ($10 != "") { printf "%.4f", $10/100 } }' \
  || true
)

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
