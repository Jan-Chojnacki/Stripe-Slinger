#!/bin/sh
set -euo pipefail

: "${CI_API_V4_URL:?CI_API_V4_URL required}"
: "${CI_PROJECT_ID:?CI_PROJECT_ID required}"

if [ -z "${CI_COMMIT_TAG:-}" ]; then
  echo "no tag, skipping release"
  exit 0
fi

: "${GITLAB_TOKEN:?GITLAB_TOKEN required}"

apk add --no-cache curl

curl --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  --data "name=$CI_COMMIT_TAG" \
  --data "tag_name=$CI_COMMIT_TAG" \
  --data "description=Release $CI_COMMIT_TAG" \
  "$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"
