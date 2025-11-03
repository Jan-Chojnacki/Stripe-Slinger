#!/bin/sh
set -euo pipefail

: "${CI_RUST_VERSION:?required}"
: "${CI_REGISTRY_IMAGE:?required}"
: "${CI_REGISTRY:?required}"
: "${CI_REGISTRY_USER:=gitlab-ci-token}"
: "${CI_REGISTRY_PASSWORD:=$CI_JOB_TOKEN}"
: "${RUNTIME_IMAGE_REPO:=$CI_REGISTRY_IMAGE/runtime}"
: "${RUNTIME_BASE_IMAGE:=rust:${CI_RUST_VERSION}}"

FILES="$(
  { find .gitlab/ci/images/runtime -type f -print;
    echo .gitlab/ci/runtime-image.yml;
    echo .gitlab/ci/scripts/build-runtime-image.sh; } | LC_ALL=C sort
)"
FILES_HASH="$(cat $FILES | sha256sum | cut -c1-16)"

docker login -u "$CI_REGISTRY_USER" -p "$CI_REGISTRY_PASSWORD" "$CI_REGISTRY" >/dev/null 2>&1 || true
docker pull "$RUNTIME_BASE_IMAGE" >/dev/null
BASE_REPO_DIGEST="$(docker inspect --format='{{index .RepoDigests 0}}' "$RUNTIME_BASE_IMAGE")"
BASE_DIGEST="${BASE_REPO_DIGEST##*@}"   
BASE_SHORT="$(echo "$BASE_DIGEST" | cut -c8-19)"

KEY="$(printf '%s|%s|%s' "$CI_RUST_VERSION" "$BASE_DIGEST" "$FILES_HASH" | sha256sum | cut -c1-12)"
IMMUTABLE_TAG="${RUNTIME_IMAGE_REPO}:${CI_RUST_VERSION}-${BASE_SHORT}-${KEY}"
MOVING_TAG="${RUNTIME_IMAGE_REPO}:rust-${CI_RUST_VERSION}"

echo "Base:        $RUNTIME_BASE_IMAGE @ $BASE_DIGEST"
echo "Files hash:  $FILES_HASH"
echo "Fingerprint: $KEY"
echo "Target tag:  $IMMUTABLE_TAG"

if docker manifest inspect "$IMMUTABLE_TAG" >/dev/null 2>&1; then
  echo "Image already present. Skipping build."
  exit 0
fi

docker build \
  --pull \
  --build-arg RUST_VERSION="$CI_RUST_VERSION" \
  -f .gitlab/ci/images/runtime/Dockerfile \
  -t "$IMMUTABLE_TAG" \
  .gitlab/ci/images/runtime

docker push "$IMMUTABLE_TAG"

docker tag "$IMMUTABLE_TAG" "$MOVING_TAG"
docker push "$MOVING_TAG"

echo "Pushed: $IMMUTABLE_TAG and $MOVING_TAG"
