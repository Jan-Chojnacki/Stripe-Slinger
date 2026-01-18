FROM golang:1.25-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
  protobuf-compiler \
  libprotobuf-dev \
  && rm -rf /var/lib/apt/lists/*

ARG PROTOC_GEN_GO_VERSION=v1.36.6
ARG PROTOC_GEN_GO_GRPC_VERSION=v1.5.1
RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@${PROTOC_GEN_GO_VERSION} \
  && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@${PROTOC_GEN_GO_GRPC_VERSION}

WORKDIR /src

COPY services/metrics-gateway/go.mod services/metrics-gateway/go.sum ./services/metrics-gateway/

WORKDIR /src/services/metrics-gateway
RUN go mod download

COPY api/proto /src/api/proto
COPY services/metrics-gateway .

RUN chmod +x ./scripts/gen-proto.sh && ./scripts/gen-proto.sh

RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o metrics-gateway ./cmd/metrics-gateway

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app

COPY --from=build /src/services/metrics-gateway/metrics-gateway /app/metrics-gateway

EXPOSE 8080

ENTRYPOINT ["/app/metrics-gateway"]
