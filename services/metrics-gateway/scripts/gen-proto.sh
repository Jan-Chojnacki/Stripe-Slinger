#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd "$(dirname "$0")/../../.." && pwd)"
GW_DIR="$ROOT_DIR/services/metrics-gateway"

PROTO_DIR="$ROOT_DIR/api/proto"

OUT_DIR="$GW_DIR/internal/pb"
PROTO_FILE="$PROTO_DIR/metrics/v1/ingest.proto"

echo "ROOT_DIR=$ROOT_DIR"
echo "PROTO_DIR=$PROTO_DIR"
echo "PROTO_FILE=$PROTO_FILE"
echo "OUT_DIR=$OUT_DIR"

if [ ! -f "$PROTO_FILE" ]; then
  echo "ERROR: proto file not found: $PROTO_FILE" >&2
  exit 1
fi

command -v protoc >/dev/null 2>&1 || { echo "ERROR: protoc not found" >&2; exit 1; }
command -v protoc-gen-go >/dev/null 2>&1 || { echo "ERROR: protoc-gen-go not found" >&2; exit 1; }
command -v protoc-gen-go-grpc >/dev/null 2>&1 || { echo "ERROR: protoc-gen-go-grpc not found" >&2; exit 1; }

rm -rf "$OUT_DIR/metrics/v1"
mkdir -p "$OUT_DIR"

echo "Generating Go protobuf/grpc into: $OUT_DIR"
protoc \
  -I="$PROTO_DIR" \
  -I="/usr/include" \
  -I="/usr/local/include" \
  --go_out="$OUT_DIR" --go_opt=paths=source_relative \
  --go-grpc_out="$OUT_DIR" --go-grpc_opt=paths=source_relative \
  "$PROTO_FILE"

if command -v gofmt >/dev/null 2>&1; then
  find "$OUT_DIR" -name '*.go' -print0 | xargs -0 gofmt -w
fi

echo "OK"
