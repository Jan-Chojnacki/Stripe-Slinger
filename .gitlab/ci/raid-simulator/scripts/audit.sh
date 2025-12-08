#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-audit] Starting cargo audit job..."
echo "[raid-simulator-audit] Using manifest: ${RUST_PROJECT_DIR}/Cargo.toml"

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  echo "[raid-simulator-audit] Strict mode: failing on any vulnerabilities..."
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings
  echo "[raid-simulator-audit] cargo audit passed (strict)."
else
  echo "[raid-simulator-audit] Soft mode: reporting vulnerabilities but not failing the job..."
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings || {
      echo "[raid-simulator-audit] Vulnerabilities detected (soft mode). Job continues."
    }
fi

echo "[raid-simulator-audit] Audit job completed."
