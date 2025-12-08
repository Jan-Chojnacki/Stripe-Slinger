#!/bin/sh
set -eu

: "${CI_API_V4_URL:?CI_API_V4_URL required}"
: "${CI_PROJECT_ID:?CI_PROJECT_ID required}"

echo "[release] Starting GitLab release job..."

if [ -z "${CI_COMMIT_TAG:-}" ]; then
  echo "[release] No tag detected (CI_COMMIT_TAG is empty); skipping release."
  exit 0
fi

: "${GITLAB_TOKEN:?GITLAB_TOKEN required}"

echo "[release] Installing curl in release image..."
apk add --no-cache curl >/dev/null

echo "[release] Creating GitLab release for tag ${CI_COMMIT_TAG}..."
curl --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  --data "name=$CI_COMMIT_TAG" \
  --data "tag_name=$CI_COMMIT_TAG" \
  --data "description=Release $CI_COMMIT_TAG" \
  "$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"

echo "[release] GitLab release request sent for tag ${CI_COMMIT_TAG}."
echo "[release] Release job completed."
