FROM rust:1.91-bookworm AS build

WORKDIR /src

RUN apt-get update && apt-get install -y --no-install-recommends \
  protobuf-compiler \
  libprotobuf-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY proto ./proto
COPY services/raid-simulator ./services/raid-simulator

WORKDIR /src/services/raid-simulator
RUN cargo build --release -p raid-cli --locked


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  fuse3 \
  samba \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
  if [ -f /etc/fuse.conf ]; then \
  sed -i 's/^[[:space:]]*#\?[[:space:]]*user_allow_other[[:space:]]*$/user_allow_other/' /etc/fuse.conf; \
  grep -q '^user_allow_other$' /etc/fuse.conf || echo 'user_allow_other' >> /etc/fuse.conf; \
  else \
  echo 'user_allow_other' > /etc/fuse.conf; \
  fi

RUN set -eux; \
  grep -qE '^nonroot:' /etc/group || echo 'nonroot:x:65532:' >> /etc/group; \
  grep -qE '^nonroot:' /etc/passwd || echo 'nonroot:x:65532:65532:nonroot:/nonexistent:/usr/sbin/nologin' >> /etc/passwd; \
  mkdir -p /mnt/raid /disks /sockets /var/run/samba; \
  chown -R 65532:65532 /mnt/raid /disks /sockets

WORKDIR /app
COPY --from=build /src/services/raid-simulator/target/release/raid-cli /usr/local/bin/raid-cli

COPY services/raid-simulator/config/samba/smb.conf /etc/samba/smb.conf
COPY services/raid-simulator/scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

USER 0:0
ENTRYPOINT ["/entrypoint.sh"]
