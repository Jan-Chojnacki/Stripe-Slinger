#!/bin/sh
set -eu

: "${DEBIAN_SNAPSHOT:?DEBIAN_SNAPSHOT is required}"
: "${RUST_VERSION:?RUST_VERSION is required}"
: "${NEXTEST_VERSION:?NEXTEST_VERSION is required}"
: "${LLVM_COV_VERSION:?LLVM_COV_VERSION is required}"
: "${CARGO_AUDIT_VERSION:?CARGO_AUDIT_VERSION is required}"

echo "[rust-runtime-setup] Starting Rust CI runtime setup..."

echo "[rust-runtime-setup] Configuring Debian APT snapshot sources..."
rm -f /etc/apt/sources.list.d/debian.sources
echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" > /etc/apt/sources.list
echo "deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" >> /etc/apt/sources.list

echo "[rust-runtime-setup] Running apt-get update..."
apt-get -o Acquire::Check-Valid-Until=false update

echo "[rust-runtime-setup] Installing base system packages for Rust CI runtime..."
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  curl \
  jq \
  libssl-dev \
  pkg-config \
  util-linux

echo "[rust-runtime-setup] Cleaning APT cache..."
rm -rf /var/lib/apt/lists/*

echo "[rust-runtime-setup] Creating CI user 'ci' and home directories..."
useradd -m -u 1000 ci
mkdir -p /home/ci/.cargo /home/ci/.rustup /home/ci/target
chown -R ci:ci /home/ci

echo "[rust-runtime-setup] Preparing Rust toolchain directories..."
install -d -o ci -g ci /home/ci/.cargo /home/ci/.rustup

echo "[rust-runtime-setup] Downloading rustup installer..."
curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
chown ci:ci /tmp/rustup-init.sh

echo "[rust-runtime-setup] Installing Rust toolchain via rustup..."
su -s /bin/sh -c "sh /tmp/rustup-init.sh -y --default-toolchain ${RUST_VERSION} --profile minimal" ci
rm /tmp/rustup-init.sh

echo "[rust-runtime-setup] Adding rustfmt, clippy and llvm-tools-preview components..."
su -s /bin/sh -c "rustup default ${RUST_VERSION} && rustup component add --toolchain ${RUST_VERSION} rustfmt clippy llvm-tools-preview" ci

echo "[rust-runtime-setup] Installing cargo-nextest..."
su -s /bin/sh -c "cargo install --locked cargo-nextest --version ${NEXTEST_VERSION}" ci

echo "[rust-runtime-setup] Installing cargo-llvm-cov..."
su -s /bin/sh -c "cargo install --locked cargo-llvm-cov --version ${LLVM_COV_VERSION}" ci

echo "[rust-runtime-setup] Installing cargo-audit..."
su -s /bin/sh -c "cargo install --locked cargo-audit --version ${CARGO_AUDIT_VERSION}" ci

echo "[rust-runtime-setup] Rust CI runtime setup completed."
