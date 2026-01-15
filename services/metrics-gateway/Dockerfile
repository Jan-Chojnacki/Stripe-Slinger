FROM golang:1.25 AS build

WORKDIR /src

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

ARG PROTOC_GEN_GO_VERSION=v1.36.6
ARG PROTOC_GEN_GO_GRPC_VERSION=v1.5.1

RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@${PROTOC_GEN_GO_VERSION} \
 && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@${PROTOC_GEN_GO_GRPC_VERSION}

# ZMIANA: Kopiujemy nową strukturę api/proto
COPY api/proto ./api/proto

# Kopiujemy kod serwisu
COPY services/metrics-gateway ./services/metrics-gateway

WORKDIR /src/services/metrics-gateway
RUN go mod download

# Skrypt gen-proto.sh (który poprawiliśmy wcześniej) szuka plików w ../../api/proto.
# Dzięki strukturze skopiowanej powyżej, znajdzie je bez problemu.
RUN chmod +x ./scripts/gen-proto.sh && ./scripts/gen-proto.sh

# Budujemy aplikację.
# Upewnij się, że ścieżka ./cmd/metrics-gateway wskazuje na folder zawierający main.go
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o metrics-gateway ./cmd/metrics-gateway

FROM gcr.io/distroless/base-debian12:nonroot
WORKDIR /app
COPY --from=build /src/services/metrics-gateway/metrics-gateway /app/metrics-gateway

EXPOSE 8080
ENTRYPOINT ["/app/metrics-gateway"]
