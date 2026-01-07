FROM rust:1.85 AS build

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
WORKDIR /app
COPY --from=build /src/services/raid-simulator/target/release/raid-cli /usr/local/bin/raid-cli

USER 65532:65532

ENTRYPOINT ["raid-cli"]
