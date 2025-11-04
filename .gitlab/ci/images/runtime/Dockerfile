ARG RUST_VERSION=1.91
FROM rust:${RUST_VERSION}

# --- system deps (root) ---
ARG DEBIAN_SNAPSHOT=20240601T000000Z
RUN set -eux; \
    rm -f /etc/apt/sources.list.d/debian.sources; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" > /etc/apt/sources.list; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" >> /etc/apt/sources.list; \
    apt-get -o Acquire::Check-Valid-Until=false update; \
    apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev ca-certificates jq curl \
    && rm -rf /var/lib/apt/lists/*

# --- user + katalogi (root) ---
RUN useradd -m -u 1000 ci \
 && mkdir -p /home/ci/.cargo /home/ci/.rustup /home/ci/target \
 && chown -R ci:ci /home/ci

# --- per-user env (user) ---
USER ci
ENV CARGO_HOME=/home/ci/.cargo \
    RUSTUP_HOME=/home/ci/.rustup \
    CARGO_TARGET_DIR=/home/ci/target \
    PATH=/home/ci/.cargo/bin:$PATH \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# --- rustup + toolchain + komponenty (user) ---
# Uwaga: poprawna sekwencja (bez '--component' w 'toolchain install')
RUN set -eux; \
    curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh; \
    sh /tmp/rustup-init.sh -y --default-toolchain ${RUST_VERSION} --profile minimal; \
    rm /tmp/rustup-init.sh; \
    rustup default ${RUST_VERSION}; \
    rustup component add --toolchain ${RUST_VERSION} rustfmt clippy llvm-tools-preview

# --- narzędzia cargo (user) ---
ARG NEXTEST_VERSION=0.9.110
RUN cargo install --locked cargo-nextest --version ${NEXTEST_VERSION}

ARG LLVM_COV_VERSION=0.6.21
RUN cargo install --locked cargo-llvm-cov --version ${LLVM_COV_VERSION}

ARG CARGO_AUDIT_VERSION=0.21.2
RUN cargo install --locked cargo-audit --version ${CARGO_AUDIT_VERSION}
