#!/bin/sh
set -euo pipefail

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || { [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] && [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ]; }; then
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings
else
  cargo audit \
    --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
    --deny warnings || true
fi
