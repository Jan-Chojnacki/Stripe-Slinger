#!/bin/sh
set -euo pipefail

: "${CI_REGISTRY_IMAGE:?CI_REGISTRY_IMAGE required}"
: "${CI_COMMIT_SHORT_SHA:?CI_COMMIT_SHORT_SHA required}"
: "${CI_REGISTRY:?CI_REGISTRY required}"
: "${CI_REGISTRY_USER:?CI_REGISTRY_USER required}"
: "${CI_REGISTRY_PASSWORD:?CI_REGISTRY_PASSWORD required}"

IMAGE_SHA="$CI_REGISTRY_IMAGE:ci-$CI_COMMIT_SHORT_SHA"

docker build -t "$IMAGE_SHA" .

echo "$CI_REGISTRY_PASSWORD" | docker login -u "$CI_REGISTRY_USER" --password-stdin "$CI_REGISTRY"

docker push "$IMAGE_SHA"

if [ -z "${CI_COMMIT_TAG:-}" ]; then
  docker tag "$IMAGE_SHA" "$CI_REGISTRY_IMAGE:latest"
  docker push "$CI_REGISTRY_IMAGE:latest"
else
  docker tag "$IMAGE_SHA" "$CI_REGISTRY_IMAGE:$CI_COMMIT_TAG"
  docker push "$CI_REGISTRY_IMAGE:$CI_COMMIT_TAG"
fi
