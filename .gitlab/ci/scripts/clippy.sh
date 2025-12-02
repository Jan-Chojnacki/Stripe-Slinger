#!/bin/bash
set -euo pipefail

cargo clippy --workspace --all-targets --all-features \
  -- -D clippy::correctness -D clippy::suspicious -A dead_code -A unused_variables -A unused_mut
