#!/usr/bin/env bash
set -euo pipefail

if [ "${CI_COMMIT_BRANCH:-}" = "master" ] || { [ "${CI_PIPELINE_SOURCE:-}" = "merge_request_event" ] && [ "${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}" = "master" ]; }; then
  cargo audit --deny warnings
else
  cargo audit --deny warnings || true
fi
