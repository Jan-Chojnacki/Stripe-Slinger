#!/bin/sh
set -eu

: "${CI_RUST_VERSION:?required}"
: "${CI_REGISTRY_IMAGE:?required}"
: "${CI_REGISTRY:?required}"
: "${CI_JOB_TOKEN:?required}"

RUNTIME_IMAGE_REPO="${RUNTIME_IMAGE_REPO:-$CI_REGISTRY_IMAGE/rust-runtime}"
RUNTIME_BASE_IMAGE="${RUNTIME_BASE_IMAGE:-rust:${CI_RUST_VERSION}}"

echo "[rust-runtime-image] Starting Rust runtime image build..."
echo "[rust-runtime-image] Using base image: ${RUNTIME_BASE_IMAGE}"
echo "[rust-runtime-image] Target repo: ${RUNTIME_IMAGE_REPO}"

FILES_HASH="$(
  {
    find .gitlab/ci/images/rust-runtime -type f -print
    echo .gitlab/ci/shared/jobs/runtime-image.yml
    echo .gitlab/ci/shared/scripts/build-rust-runtime-image.sh
  } | LC_ALL=C sort |
  while IFS= read -r f; do
    cat "$f"
  done | sha256sum | cut -c1-16
)"

echo "[rust-runtime-image] Calculated files hash: ${FILES_HASH}"

echo "[rust-runtime-image] Logging in to registry..."
echo "$CI_JOB_TOKEN" | docker login -u gitlab-ci-token --password-stdin "$CI_REGISTRY" >/dev/null

echo "[rust-runtime-image] Probing base image digest (metadata only)..."

BASE_DIGEST=$(docker manifest inspect "$RUNTIME_BASE_IMAGE" -v | grep "Descriptor" -A 5 | grep "digest" | head -n1 | cut -d'"' -f4 || echo "")

if [ -z "$BASE_DIGEST" ]; then
  echo "WARNING: Could not fetch digest via manifest inspect. Falling back to quick pull..."
  docker pull "$RUNTIME_BASE_IMAGE" >/dev/null
  BASE_REPO_DIGEST="$(docker inspect --format='{{index .RepoDigests 0}}' "$RUNTIME_BASE_IMAGE" 2>/dev/null || true)"
  BASE_DIGEST="${BASE_REPO_DIGEST##*@}"
fi

BASE_SHORT="$(printf '%s' "$BASE_DIGEST" | cut -c8-19)"
KEY="$(printf '%s|%s|%s' "$CI_RUST_VERSION" "$BASE_DIGEST" "$FILES_HASH" | sha256sum | cut -c1-12)"
IMMUTABLE_TAG="${RUNTIME_IMAGE_REPO}:${CI_RUST_VERSION}-${BASE_SHORT}-${KEY}"
MOVING_TAG="${RUNTIME_IMAGE_REPO}:rust-${CI_RUST_VERSION}"

echo "[rust-runtime-image] Base digest: ${BASE_DIGEST}"
echo "[rust-runtime-image] Fingerprint key: ${KEY}"
echo "[rust-runtime-image] Immutable tag: ${IMMUTABLE_TAG}"
echo "[rust-runtime-image] Moving tag:    ${MOVING_TAG}"

manifest_digest() {
  docker manifest inspect "$1" 2>/dev/null | sed -n 's/.*"digest": *"\(sha256:[a-f0-9]\+\)".*/\1/p' | head -n1
}

IMM_EXISTS=false
if docker manifest inspect "$IMMUTABLE_TAG" >/dev/null 2>&1; then
  IMM_EXISTS=true
fi

if [ "$IMM_EXISTS" = true ]; then
  echo "[rust-runtime-image] Immutable image already exists: ${IMMUTABLE_TAG}"
  MOV_DIGEST="$(manifest_digest "$MOVING_TAG" || true)"
  IMM_DIGEST="$(manifest_digest "$IMMUTABLE_TAG" || true)"

  if [ -n "${MOV_DIGEST:-}" ] && [ "$MOV_DIGEST" = "$IMM_DIGEST" ]; then
    echo "[rust-runtime-image] Moving tag already points to immutable image. Skipping build."
    exit 0
  fi

  echo "[rust-runtime-image] Retagging moving alias to existing immutable image..."
  docker pull "$IMMUTABLE_TAG" >/dev/null
  docker tag "$IMMUTABLE_TAG" "$MOVING_TAG"
  docker push "$MOVING_TAG"
  exit 0
fi

echo "[rust-runtime-image] Building new immutable runtime image..."
docker build --pull \
  --build-arg RUST_VERSION="$CI_RUST_VERSION" \
  --build-arg CARGO_AUDIT_VERSION="latest" \
  -f .gitlab/ci/images/rust-runtime/Dockerfile \
  -t "$IMMUTABLE_TAG" \
  .gitlab/ci/images/rust-runtime

echo "[rust-runtime-image] Pushing immutable image ${IMMUTABLE_TAG}..."
docker push "$IMMUTABLE_TAG"

echo "[rust-runtime-image] Tagging and pushing moving alias ${MOVING_TAG}..."
docker tag "$IMMUTABLE_TAG" "$MOVING_TAG"
docker push "$MOVING_TAG"

echo "[rust-runtime-image] Build completed successfully."
