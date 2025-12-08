FROM golang:1.25 AS build

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o metrics-gateway ./cmd/metrics-gateway

FROM gcr.io/distroless/base-debian12

WORKDIR /app
COPY --from=build /app/metrics-gateway /app/metrics-gateway

EXPOSE 8080

ENTRYPOINT ["/app/metrics-gateway"]
