#!/bin/sh
set -eu

echo "[no-code-gate] Starting no-code gate check..."

echo "[no-code-gate] Non-code change detected; skipping language-specific CI pipelines (Rust/Go)."

echo "[no-code-gate] No-code gate check completed."
