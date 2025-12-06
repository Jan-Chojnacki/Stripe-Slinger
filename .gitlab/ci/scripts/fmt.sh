#!/bin/sh
set -euo pipefail

cargo fmt \
  --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" \
  --all -- --check
