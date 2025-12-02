ARG RUST_VERSION=1.91.0
FROM rust:${RUST_VERSION}

ARG DEBIAN_SNAPSHOT=20240601T000000Z
RUN set -eux; \
    rm -f /etc/apt/sources.list.d/debian.sources; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" > /etc/apt/sources.list; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" >> /etc/apt/sources.list; \
    apt-get -o Acquire::Check-Valid-Until=false update; \
    apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev ca-certificates jq curl \
    util-linux \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 ci \
    && mkdir -p /home/ci/.cargo /home/ci/.rustup /home/ci/target \
    && chown -R ci:ci /home/ci

ENV CARGO_HOME=/home/ci/.cargo \
    RUSTUP_HOME=/home/ci/.rustup \
    PATH=/home/ci/.cargo/bin:$PATH \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN set -eux; \
    install -d -o ci -g ci /home/ci/.cargo /home/ci/.rustup; \
    curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh; \
    chown ci:ci /tmp/rustup-init.sh; \
    su -s /bin/sh -c 'sh /tmp/rustup-init.sh -y --default-toolchain '"${RUST_VERSION}"' --profile minimal' ci; \
    rm /tmp/rustup-init.sh; \
    su -s /bin/sh -c 'rustup default '"${RUST_VERSION}"' && rustup component add --toolchain '"${RUST_VERSION}"' rustfmt clippy llvm-tools-preview' ci

ARG NEXTEST_VERSION=0.9.110
RUN su -s /bin/sh -c 'cargo install --locked cargo-nextest --version '"${NEXTEST_VERSION}" ci

ARG LLVM_COV_VERSION=0.6.21
RUN su -s /bin/sh -c 'cargo install --locked cargo-llvm-cov --version '"${LLVM_COV_VERSION}" ci

ARG CARGO_AUDIT_VERSION=0.21.2
RUN su -s /bin/sh -c 'cargo install --locked cargo-audit --version '"${CARGO_AUDIT_VERSION}" ci

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
WORKDIR /build
