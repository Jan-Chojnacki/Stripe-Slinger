ARG GO_VERSION=1.25
FROM golang:${GO_VERSION}

RUN useradd -m -u 1000 ci

WORKDIR /build

COPY entrypoint.sh /usr/local/bin/entrypoint
RUN chmod +x /usr/local/bin/entrypoint

ENTRYPOINT ["/usr/local/bin/entrypoint"]
