#!/bin/sh
set -euo pipefail

: "${CI_RUST_VERSION:?CI_RUST_VERSION required}"
: "${CI_REGISTRY_IMAGE:?CI_REGISTRY_IMAGE required}"
: "${CI_COMMIT_SHORT_SHA:?CI_COMMIT_SHORT_SHA required}"
: "${CI_JOB_TOKEN:?CI_JOB_TOKEN required}"
: "${CI_REGISTRY:?CI_REGISTRY required}"

IMAGE_TAG="$CI_REGISTRY_IMAGE/ci:rust-$CI_RUST_VERSION-$CI_COMMIT_SHORT_SHA"
LATEST_TAG="$CI_REGISTRY_IMAGE/ci:rust-$CI_RUST_VERSION"

docker build \
  --build-arg RUST_VERSION="$CI_RUST_VERSION" \
  -f .gitlab/ci/images/runtime/Dockerfile \
  -t "$IMAGE_TAG" .

echo "$CI_JOB_TOKEN" | docker login -u gitlab-ci-token --password-stdin "$CI_REGISTRY"

docker push "$IMAGE_TAG"

docker tag "$IMAGE_TAG" "$LATEST_TAG"
docker push "$LATEST_TAG"
