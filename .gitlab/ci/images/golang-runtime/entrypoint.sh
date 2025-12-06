#!/bin/sh
set -e
umask 002

echo "[entrypoint] Starting CI entrypoint for golang runtime..."

if [ -n "${CI_PROJECT_DIR:-}" ] && [ -d "${CI_PROJECT_DIR:-}" ]; then
  echo "[entrypoint] Ensuring correct ownership for project directory..."
  chown -R ci:ci "$CI_PROJECT_DIR" || true
fi

if [ -d /builds ]; then
  echo "[entrypoint] Ensuring correct ownership for build target directories..."
  find /builds -maxdepth 3 -type d -name target -exec chown -R ci:ci {} + 2>/dev/null || true
fi

echo "[entrypoint] Switching to 'ci' user..."
exec runuser -u ci -- "$@"
