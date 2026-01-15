#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-audit] Starting cargo audit job..."
echo "[raid-simulator-audit] Switching to directory: ${RUST_PROJECT_DIR}"

cd "$RUST_PROJECT_DIR"

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  echo "[raid-simulator-audit] Strict mode: failing on any vulnerabilities..."
  cargo audit --deny warnings
  echo "[raid-simulator-audit] cargo audit passed (strict)."
else
  echo "[raid-simulator-audit] Soft mode: reporting vulnerabilities but not failing the job..."
  cargo audit --deny warnings || {
      echo "[raid-simulator-audit] Vulnerabilities detected (soft mode). Job continues."
    }
fi

echo "[raid-simulator-audit] Audit job completed."
