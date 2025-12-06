#!/bin/sh
set -euo pipefail

cargo clippy \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --workspace --all-targets --all-features \
  -- -D clippy::correctness -D clippy::suspicious -A dead_code -A unused_variables -A unused_mut
