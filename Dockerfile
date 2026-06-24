# ── Builder ──────────────────────────────────────────────────────────────────
# zood is built CGO-free, and that is a correctness decision, not a shortcut.
#
# zood transitively imports luxfi/dex/pkg/lx (via the precompile registry in
# luxfi/evm). Its `//go:build cgo && linux` files pull a CUDA order-matching
# bundle (lux-dex-amm-cuda / lux-dex-clob-cuda pkg-config + dex_swap.h). But the
# package also ships a complete pure-Go path — amm_nogpu.go (`!cgo`),
# orderbook_cuda_stub.go (`!cgo || !linux`), signed_order_nocgo.go (`!cgo`) —
# selected automatically when CGO is off. So CGO_ENABLED=0 drops the CUDA
# dependency entirely; no special builder base, no /usr/local CUDA libs.
#
# This is also the only correct build for a validator: beluga's consensus-path
# precompiles (0x0303 inference deterministic int8 zen-nano, 0x0304 aivmbridge)
# must produce byte-identical state roots on every node, so they are pure-Go by
# design — GPU/CGO non-determinism would fork consensus. Canonical luxd's own
# Makefile sanctions `CGO_ENABLED=0 make build`; zood pulls the same lever. The
# secp256k1/sr25519 crypto resolves to its pure-Go implementations (verified:
# the full binary links clean with CGO off), so nothing is lost but C/GPU
# acceleration a beluga L3 validator does not need.
#
# Net effect: a stock golang base cross-compiles zood to any arch.
ARG LUX_BUILDER_BASE=golang:1.26-bookworm
FROM --platform=$BUILDPLATFORM ${LUX_BUILDER_BASE} AS builder
RUN apt-get update && apt-get install -y --no-install-recommends git && rm -rf /var/lib/apt/lists/*
WORKDIR /build

ENV GOPRIVATE=*
ENV GONOSUMCHECK=*
ENV GONOSUMDB=*
ENV GOPROXY=direct

COPY go.mod go.sum ./
# Private modules (luxfi/dex, luxfi/precompile, zooai/*) are fetched over HTTPS
# with a build-time token supplied as a BuildKit secret (`gh_token`). When the
# secret is absent the rewrite is skipped and only public modules resolve — so
# this is safe for public-only builds. GONOSUMCHECK=*/GONOSUMDB=* bypass sum
# verification for the luxfi graph (tags get rewritten, causing go.sum drift).
RUN --mount=type=secret,id=gh_token \
    if [ -s /run/secrets/gh_token ]; then \
      git config --global \
        url."https://x-access-token:$(cat /run/secrets/gh_token)@github.com/".insteadOf \
        "https://github.com/"; \
    fi && \
    sed -i '/luxfi\//d' go.sum && go mod download

COPY . .
RUN sed -i '/luxfi\//d' go.sum

# Per SCALE_STANDARD.md §2 (https://github.com/hanzoai/hips/blob/main/docs/SCALE_STANDARD.md)
# — every Go production Dockerfile that emits JSON to a client builds with
# GOEXPERIMENT=jsonv2. Verified -12% time / -23% allocs on the edge POST
# roundtrip vs encoding/json v1.
ARG GO_EXPERIMENT=jsonv2
ENV GOEXPERIMENT=${GO_EXPERIMENT}

# Cross-compile on the native build platform (no qemu) — pure Go makes this free.
# TARGETOS/TARGETARCH are supplied by BuildKit; default to linux/amd64 when the
# builder (e.g. a plain `docker build`) does not set them.
ARG TARGETOS
ARG TARGETARCH
# Build zood — the Zoo Network node. The single binary self-installs the Zoo EVM
# (+ AI/FHE/PQ/ZK precompiles, incl. the 0x0303 inference precompile and the
# 0x0304 aivmbridge), Zoo DEX, and Zoo FHE VMs as plugin subprocesses at startup
# (see main.go installPlugin), so no plugin symlinks are needed here.
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH:-amd64} \
    go build -mod=mod -trimpath -ldflags="-w -s" -o /build/zood .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/zood /usr/local/bin/zood

# Genesis configs baked in for offline bootstrap.
#   genesis.json            — Zoo L2 EVM (chainId 200200)
#   beluga/genesis.json     — Beluga L3 EVM (evmChainId 420420), AI/FHE/PQ/ZK
#   deploy-dex/genesis.json — Zoo DEX
COPY genesis.json /etc/zoo/genesis.json
COPY beluga/genesis.json /etc/zoo/beluga-genesis.json
COPY beluga/config.json /etc/zoo/beluga-config.json
COPY deploy-dex/genesis.json /etc/zoo/dex-genesis.json

ENTRYPOINT ["zood"]
