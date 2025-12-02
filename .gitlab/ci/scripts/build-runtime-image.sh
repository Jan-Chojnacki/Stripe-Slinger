#!/bin/sh
set -euo pipefail

: "${CI_RUST_VERSION:?required}"
: "${CI_REGISTRY_IMAGE:?required}"
: "${CI_REGISTRY:?required}"
: "${CI_JOB_TOKEN:?required}"

RUNTIME_IMAGE_REPO="${RUNTIME_IMAGE_REPO:-$CI_REGISTRY_IMAGE/runtime}"
RUNTIME_BASE_IMAGE="${RUNTIME_BASE_IMAGE:-rust:${CI_RUST_VERSION}}"

FILES_HASH="$(
  {
    find .gitlab/ci/images/runtime -type f -print
    echo .gitlab/ci/runtime-image.yml
    echo .gitlab/ci/scripts/build-runtime-image.sh
  } | LC_ALL=C sort |
  while IFS= read -r f; do
    cat "$f"
  done | sha256sum | cut -c1-16
)"

echo "$CI_JOB_TOKEN" | docker login -u gitlab-ci-token --password-stdin "$CI_REGISTRY" >/dev/null

docker pull "$RUNTIME_BASE_IMAGE" >/dev/null 2>&1 || true
BASE_REPO_DIGEST="$(docker inspect --format='{{index .RepoDigests 0}}' "$RUNTIME_BASE_IMAGE" 2>/dev/null || true)"
BASE_DIGEST="${BASE_REPO_DIGEST##*@}"
BASE_SHORT="$(printf '%s' "$BASE_DIGEST" | cut -c8-19)"

KEY="$(printf '%s|%s|%s' "$CI_RUST_VERSION" "$BASE_DIGEST" "$FILES_HASH" | sha256sum | cut -c1-12)"
IMMUTABLE_TAG="${RUNTIME_IMAGE_REPO}:${CI_RUST_VERSION}-${BASE_SHORT}-${KEY}"
MOVING_TAG="${RUNTIME_IMAGE_REPO}:rust-${CI_RUST_VERSION}"

echo "Base: $RUNTIME_BASE_IMAGE @ $BASE_DIGEST"
echo "Files: $FILES_HASH"
echo "Fingerprint: $KEY"
echo "Immutable: $IMMUTABLE_TAG"
echo "Moving:    $MOVING_TAG"

manifest_digest() {
  docker manifest inspect "$1" 2>/dev/null | sed -n 's/.*"digest": *"\(sha256:[a-f0-9]\+\)".*/\1/p' | head -n1
}

IMM_EXISTS=false
if docker manifest inspect "$IMMUTABLE_TAG" >/dev/null 2>&1; then
  IMM_EXISTS=true
fi

if [ "$IMM_EXISTS" = true ]; then
  MOV_DIGEST="$(manifest_digest "$MOVING_TAG" || true)"
  IMM_DIGEST="$(manifest_digest "$IMMUTABLE_TAG" || true)"
  if [ -n "${MOV_DIGEST:-}" ] && [ "$MOV_DIGEST" = "$IMM_DIGEST" ]; then
    echo "Up to date (alias == immutable). Skipping build."
    exit 0
  fi
  docker pull "$IMMUTABLE_TAG" >/dev/null 2>&1 || true
  docker tag "$IMMUTABLE_TAG" "$MOVING_TAG"
  docker push "$MOVING_TAG"
  echo "Retagged moving alias to existing immutable."
  exit 0
fi

docker build --pull \
  --build-arg RUST_VERSION="$CI_RUST_VERSION" \
  -f .gitlab/ci/images/runtime/Dockerfile \
  -t "$IMMUTABLE_TAG" \
  .gitlab/ci/images/runtime

docker push "$IMMUTABLE_TAG"
docker tag "$IMMUTABLE_TAG" "$MOVING_TAG"
docker push "$MOVING_TAG"

echo "Pushed: $IMMUTABLE_TAG and updated $MOVING_TAG"
