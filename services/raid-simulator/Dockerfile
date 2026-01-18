FROM rust:1.91-bookworm AS build

WORKDIR /src

COPY api/proto ./api/proto
COPY services/raid-simulator ./services/raid-simulator

WORKDIR /src/services/raid-simulator
RUN cargo build --release -p raid-cli --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  fuse3 \
  nfs-kernel-server \
  rpcbind \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && sed -i 's/#user_allow_other/user_allow_other/' /etc/fuse.conf \
  && mkdir -p /var/lib/nfs /var/run/rpcbind /run/sendsigs.omit.d \
  && mkdir -p /mnt/raid /disks /sockets

COPY --from=build /src/services/raid-simulator/target/release/raid-cli /usr/local/bin/raid-cli
COPY services/raid-simulator/scripts/entrypoint.sh /entrypoint.sh

RUN chmod +x /entrypoint.sh

EXPOSE 2049/tcp

ENTRYPOINT ["/entrypoint.sh"]
