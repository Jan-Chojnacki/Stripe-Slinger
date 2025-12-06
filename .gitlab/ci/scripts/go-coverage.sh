#!/bin/sh
set -euo pipefail

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

if [ ! -f coverage.out ]; then
  echo "[metrics-gateway-coverage] coverage.out not found; did metrics-gateway-tests run and publish artifacts?"
  exit 1
fi

echo "[metrics-gateway-coverage] Computing coverage from coverage.out..."

report_out="$(go tool cover -func=coverage.out || true)"
printf '%s\n' "$report_out"

rate="$(
  printf '%s\n' "$report_out" \
  | awk '/^total:/ { gsub("%","",$3); if ($3 != "") { printf "%.4f", $3/100 } }' \
  || true
)"

[ -z "${rate:-}" ] && rate=0

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  awk -v r="$rate" 'BEGIN {
    if (r < 0.5) { printf "[metrics-gateway-coverage] Go coverage %.2f < 0.50. FAIL\n", r; exit 1 }
    else         { printf "[metrics-gateway-coverage] Go coverage %.2f >= 0.50. OK\n", r }
  }'
else
  echo "[metrics-gateway-coverage] Soft Go coverage check (branch): $rate"
fi
