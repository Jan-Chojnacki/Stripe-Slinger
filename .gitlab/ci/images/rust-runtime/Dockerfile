ARG RUST_VERSION=1.91.0
FROM rust:${RUST_VERSION}

ARG RUST_VERSION
ARG DEBIAN_SNAPSHOT=20240601T000000Z
ARG NEXTEST_VERSION=0.9.110
ARG LLVM_COV_VERSION=0.6.21
ARG CARGO_AUDIT_VERSION=0.21.2

ENV CARGO_HOME=/home/ci/.cargo \
    RUSTUP_HOME=/home/ci/.rustup \
    PATH=/home/ci/.cargo/bin:$PATH \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

WORKDIR /build

COPY setup-rust-runtime.sh /usr/local/bin/setup-rust-runtime
COPY entrypoint.sh         /usr/local/bin/entrypoint

RUN chmod +x /usr/local/bin/setup-rust-runtime /usr/local/bin/entrypoint \
    && /usr/local/bin/setup-rust-runtime

ENTRYPOINT ["/usr/local/bin/entrypoint"]
