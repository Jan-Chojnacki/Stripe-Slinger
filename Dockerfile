FROM rust:1.85 AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/raid-cli /usr/local/bin/raid-cli
ENTRYPOINT ["raid-cli"]
