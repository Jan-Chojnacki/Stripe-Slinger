#!/bin/sh
set -eu

: "${CI_REGISTRY:?CI_REGISTRY required}"
: "${CI_REGISTRY_IMAGE:?CI_REGISTRY_IMAGE required}"
: "${CI_COMMIT_SHORT_SHA:?CI_COMMIT_SHORT_SHA required}"
: "${CI_REGISTRY_USER:?CI_REGISTRY_USER required}"
: "${CI_REGISTRY_PASSWORD:?CI_REGISTRY_PASSWORD required}"

: "${IMAGE_NAME:?IMAGE_NAME required}"
: "${DOCKER_BUILD_CONTEXT:?DOCKER_BUILD_CONTEXT required}"
: "${DOCKERFILE_PATH:?DOCKERFILE_PATH required}"

IMAGE_REPO="${CI_REGISTRY_IMAGE}/${IMAGE_NAME}"
IMAGE_CI_TAG="${IMAGE_REPO}:ci-${CI_COMMIT_SHORT_SHA}"

echo "[docker-build] Starting docker image build job for ${IMAGE_NAME}..."
echo "[docker-build] Building image ${IMAGE_CI_TAG}"
echo "[docker-build]   context:   ${DOCKER_BUILD_CONTEXT}"
echo "[docker-build]   dockerfile: ${DOCKERFILE_PATH}"

docker build -t "${IMAGE_CI_TAG}" -f "${DOCKERFILE_PATH}" "${DOCKER_BUILD_CONTEXT}"

echo "[docker-build] Logging in to registry ${CI_REGISTRY}"
echo "${CI_REGISTRY_PASSWORD}" | docker login -u "${CI_REGISTRY_USER}" --password-stdin "${CI_REGISTRY}"

echo "[docker-build] Pushing CI image ${IMAGE_CI_TAG}"
docker push "${IMAGE_CI_TAG}"

if [ -n "${CI_COMMIT_TAG:-}" ]; then
  RELEASE_TAG="${CI_COMMIT_TAG}"
  echo "[docker-build] Tagging release image ${IMAGE_REPO}:${RELEASE_TAG}"
  docker tag "${IMAGE_CI_TAG}" "${IMAGE_REPO}:${RELEASE_TAG}"
  docker push "${IMAGE_REPO}:${RELEASE_TAG}"
elif [ "${CI_COMMIT_BRANCH:-}" = "master" ]; then
  echo "[docker-build] Tagging latest image ${IMAGE_REPO}:latest (master branch)"
  docker tag "${IMAGE_CI_TAG}" "${IMAGE_REPO}:latest"
  docker push "${IMAGE_REPO}:latest"
else
  echo "[docker-build] Not tagging latest or release (branch=${CI_COMMIT_BRANCH:-}, tag=${CI_COMMIT_TAG:-})"
fi

echo "[docker-build] Docker image build job completed for ${IMAGE_NAME}."
