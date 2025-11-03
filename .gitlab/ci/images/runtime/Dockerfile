ARG RUST_VERSION=1.91
FROM rust:${RUST_VERSION}

ARG DEBIAN_SNAPSHOT=20240601T000000Z
RUN set -eux; \
    rm -f /etc/apt/sources.list.d/debian.sources; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" > /etc/apt/sources.list; \
    echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" >> /etc/apt/sources.list; \
    apt-get -o Acquire::Check-Valid-Until=false update; \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        ca-certificates \
        jq \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 ci && mkdir -p /build && chown -R ci:ci /build
USER ci

ENV CARGO_HOME=/home/ci/.cargo \
    RUSTUP_HOME=/home/ci/.rustup \
    PATH=/home/ci/.cargo/bin:$PATH \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

WORKDIR /build

RUN rustup set profile minimal \
 && rustup toolchain install ${RUST_VERSION} \
 && rustup default ${RUST_VERSION} \
 && rustup component add rustfmt clippy llvm-tools-preview

ARG NEXTEST_VERSION=0.9.110
RUN cargo install cargo-nextest --locked --version ${NEXTEST_VERSION}

ARG LLVM_COV_VERSION=0.6.21
RUN cargo install cargo-llvm-cov --locked --version ${LLVM_COV_VERSION}

ARG CARGO_AUDIT_VERSION=0.21.2
RUN cargo install cargo-audit --locked --version ${CARGO_AUDIT_VERSION}
