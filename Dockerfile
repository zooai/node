FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.6.1 AS xx

FROM --platform=$BUILDPLATFORM golang:1.26-bookworm AS builder
COPY --from=xx / /
RUN apt-get update && apt-get install -y --no-install-recommends git clang lld && rm -rf /var/lib/apt/lists/*
ARG TARGETPLATFORM
RUN xx-apt-get install -y gcc libc6-dev
WORKDIR /build

ENV GOPRIVATE=*
ENV GONOSUMCHECK=*
ENV GONOSUMDB=*
ENV GOPROXY=direct

COPY go.mod go.sum ./
# Strip luxfi checksums — tags get rewritten causing checksum drift.
# GONOSUMCHECK=* + GONOSUMDB=* + GOFLAGS=-goflags bypass sum verification.
RUN sed -i '/luxfi\//d' go.sum && go mod download

COPY . .
RUN sed -i '/luxfi\//d' go.sum
RUN xx-go --wrap && \
    CGO_ENABLED=1 CGO_CFLAGS="-Wno-incompatible-pointer-types" \
    go build -mod=mod -ldflags="-w -s" -o /build/zood .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /zood/build/plugins
COPY --from=builder /build/zood /zood/build/zood
RUN ln -s /zood/build/zood /usr/local/bin/zood
RUN ln -s /zood/build/zood /zood/build/plugins/2n2njofjYvece8gZWCNnc1mqkcqfW6kbrhPRZPVzwxrSQrQ4gE && \
    ln -s /zood/build/zood /zood/build/plugins/mDVT5EWMumBp3LCqvKwuyZQeY1VXr1jvjGNAt8nL4UFiXvqXr

COPY genesis.json /etc/zoo/genesis.json
COPY cmd/deploy-dex/genesis.json /etc/zoo/dex-genesis.json

ENTRYPOINT ["zood"]
