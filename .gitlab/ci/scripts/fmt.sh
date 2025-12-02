#!/bin/sh
set -euo pipefail

cargo fmt --all -- --check
