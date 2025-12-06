#!/bin/sh
set -eu

: "${GO_PROJECT_DIR:?GO_PROJECT_DIR required}"

cd "$GO_PROJECT_DIR"

echo "[metrics-gateway-audit] Starting Go vulnerability audit job (govulncheck)..."
echo "[metrics-gateway-audit] Running govulncheck ./..."

hard_fail=false
if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  hard_fail=true
fi

if ! govulncheck ./...; then
  if [ "$hard_fail" = true ]; then
    echo "[metrics-gateway-audit] govulncheck reported vulnerabilities (hard fail on master / MR to master)."
    exit 1
  else
    echo "[metrics-gateway-audit] govulncheck reported vulnerabilities (soft fail on non-master branch). Continuing pipeline."
    exit 0
  fi
fi

echo "[metrics-gateway-audit] govulncheck OK; audit job completed."
