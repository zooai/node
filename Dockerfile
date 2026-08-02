FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.6.1 AS xx

FROM --platform=$BUILDPLATFORM golang:1.26.5-bookworm AS builder
# Pinned to the exact patch the go.mod directive needs, and GOTOOLCHAIN=auto so a
# future go.mod bump downloads its toolchain instead of hard-failing under the
# official image's GOTOOLCHAIN=local.
ENV GOTOOLCHAIN=auto
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

# Per SCALE_STANDARD.md §2 (https://github.com/hanzoai/hips/blob/main/docs/SCALE_STANDARD.md)
# — every Go production Dockerfile that emits JSON to a client builds
# with GOEXPERIMENT=jsonv2. Verified -12% time / -23% allocs on the
# edge POST roundtrip vs encoding/json v1.
ARG GO_EXPERIMENT=jsonv2
ENV GOEXPERIMENT=${GO_EXPERIMENT}

RUN xx-go --wrap && \
    CGO_ENABLED=1 CGO_CFLAGS="-Wno-incompatible-pointer-types" \
    go build -mod=mod -ldflags="-w -s" -o /build/zood .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
# zood is the Zoo Network node daemon — the name the Makefile builds (BINARY := zood).
# No plugin symlinks are baked here: main.go installPlugin() symlinks the running
# binary into nodeConfig.PluginDir for the EVM and DEX VM IDs at startup.
COPY --from=builder /build/zood /usr/local/bin/zood

# Genesis configs baked in for offline bootstrap.
#   genesis.json        — Zoo L2 EVM (chainId 200200)
#   beluga/genesis.json — Beluga L3 EVM (evmChainId 420420)
COPY genesis.json /etc/zoo/genesis.json
COPY beluga/genesis.json /etc/zoo/beluga-genesis.json
COPY beluga/config.json /etc/zoo/beluga-config.json

ENTRYPOINT ["zood"]
