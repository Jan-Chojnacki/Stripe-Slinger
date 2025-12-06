ARG GO_VERSION=1.25
FROM golang:${GO_VERSION}

RUN useradd -m -u 1000 ci

WORKDIR /build

COPY setup.sh      /usr/local/bin/setup
COPY entrypoint.sh /usr/local/bin/entrypoint

RUN chmod +x /usr/local/bin/setup /usr/local/bin/entrypoint \
    && /usr/local/bin/setup

ENTRYPOINT ["/usr/local/bin/entrypoint"]
