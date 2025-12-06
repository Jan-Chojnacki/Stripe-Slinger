#!/bin/sh
set -eu

: "${RUST_PROJECT_DIR:?RUST_PROJECT_DIR required}"

echo "[raid-simulator-audit] Running cargo audit for $RUST_PROJECT_DIR..."

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || {
     [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] &&
     [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ];
   }; then
  echo "[raid-simulator-audit] Strict mode (master / MR → master): failing on vulnerabilities or warnings"
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings
else
  echo "[raid-simulator-audit] Soft mode (non-master): reporting vulnerabilities but not failing the pipeline on warnings"
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings || true
fi

echo "[raid-simulator-audit] cargo audit finished"
