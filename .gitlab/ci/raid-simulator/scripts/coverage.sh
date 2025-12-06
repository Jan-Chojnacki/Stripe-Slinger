#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-coverage] Starting Rust coverage report job..."
echo "[raid-simulator-coverage] Preparing local coverage workspace..."

rm -rf target
mkdir -p target

if [ ! -f cov.tar.gz ]; then
  echo "[raid-simulator-coverage] ERROR: cov.tar.gz not found; did raid-simulator-tests run and publish artifacts?"
  exit 1
fi

echo "[raid-simulator-coverage] Extracting cov.tar.gz into target/..."
tar xzf cov.tar.gz -C target

export CARGO_TARGET_DIR=target/llvm-cov-target
export LLVM_COV_FLAGS="${LLVM_COV_FLAGS:-} -use-color=0"

mkdir -p reports

echo "[raid-simulator-coverage] Running cargo llvm-cov report for $RUST_PROJECT_DIR..."
report_out=$(cargo llvm-cov report \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --ignore-filename-regex='/.cargo/|/rustc/')

printf '%s\n' "$report_out"

rate=$(
  printf '%s\n' "$report_out" \
  | awk '/^TOTAL/ { gsub("%","",$10); if ($10 != "") { printf "%.4f", $10/100 } }' \
  || true
)

[ -z "${rate:-}" ] && rate=0

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  awk -v r="$rate" 'BEGIN {
    if (r < 0.5) { printf "[raid-simulator-coverage] Coverage %.2f < 0.50. FAIL\n", r; exit 1 }
    else         { printf "[raid-simulator-coverage] Coverage %.2f >= 0.50. OK\n", r }
  }'
else
  echo "[raid-simulator-coverage] Soft coverage check (branch): $rate"
fi

echo "[raid-simulator-coverage] Coverage job completed."
