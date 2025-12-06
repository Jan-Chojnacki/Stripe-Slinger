ARG GO_VERSION=1.25
FROM golang:${GO_VERSION}

RUN useradd -m -u 1000 ci

WORKDIR /build

RUN printf '%s\n' \
    '#!/bin/sh' \
    'set -e' \
    'umask 002' \
    'if [ -n "$CI_PROJECT_DIR" ] && [ -d "$CI_PROJECT_DIR" ]; then' \
    '  chown -R ci:ci "$CI_PROJECT_DIR" || true' \
    'fi' \
    'if [ -d /builds ]; then' \
    '  find /builds -maxdepth 3 -type d -name target -exec chown -R ci:ci {} + 2>/dev/null || true' \
    'fi' \
    'exec runuser -u ci -- "$@"' \
    > /usr/local/bin/ci-entrypoint && chmod +x /usr/local/bin/ci-entrypoint

ENTRYPOINT ["/usr/local/bin/ci-entrypoint"]
