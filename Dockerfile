# ── Builder ──────────────────────────────────────────────────────────────────
# BUILD PREREQUISITE (the one open gate for ghcr.io/zooai/node):
# zood links luxfi/dex/pkg/lx (pulled transitively by luxfi/evm/plugin/evm).
# Its //go:build cgo && linux files unconditionally `#cgo pkg-config:
# lux-dex-amm-cuda lux-dex-clob-cuda lux-crypto-secp256k1` and #include
# "amm_xyk_driver.h" / <lux/cuda/dex_swap.h> — there is NO CPU fallback when
# CGO is on under linux (and CGO is mandatory for the sr25519/secp256k1 crypto).
# So the build host MUST provide the luxcpp dex+crypto CUDA dev bundle on the
# default cgo search paths:
#   /usr/local/include/{amm_xyk_driver.h, lux/cuda/dex_swap.h, ...}
#   /usr/local/lib/{libdex_clob_cuda, libdex_amm_cuda, libsecp256k1_cpu, ...}
#   /usr/local/lib/pkgconfig/{lux-dex-amm-cuda,lux-dex-clob-cuda,lux-crypto-secp256k1}.pc
# produced by `luxcpp/dex` + `luxcpp/crypto` built with the CUDA toolchain
# (the canonical GB10 / CUDA-13 arc build env). A stock golang:bookworm CANNOT
# link zood — verified: cgo fails on `lux-dex-amm-cuda not found`, then
# `amm_xyk_driver.h: No such file`, then `lux/cuda/dex_swap.h: No such file`.
#
# UNBLOCK: publish that bundle as an installable builder base (e.g.
# ghcr.io/luxfi/dex-cuda-builder) — it does NOT exist yet (404) — then set
# LUX_BUILDER_BASE to it. Until then the image build runs on an arc node that
# has luxcpp/dex + luxcpp/crypto installed to /usr/local. golang:1.26-bookworm
# is the correct base for everything else (precompiles incl. 0x0303 inference +
# 0x0304 aivmbridge link cleanly — proven by the standalone smoke build).
ARG LUX_BUILDER_BASE=golang:1.26-bookworm
FROM ${LUX_BUILDER_BASE} AS builder
RUN apt-get update && apt-get install -y --no-install-recommends git clang lld gcc libc6-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /build

ENV GOPRIVATE=*
ENV GONOSUMCHECK=*
ENV GONOSUMDB=*
ENV GOPROXY=direct
# Surface the luxcpp dex+crypto CUDA pkg-config bundle to cgo (present on the
# CUDA-equipped builder base; harmless if the dir is empty).
ENV PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
ENV LD_LIBRARY_PATH=/usr/local/lib

COPY go.mod go.sum ./
# Strip luxfi checksums — tags get rewritten causing checksum drift.
# GONOSUMCHECK=* + GONOSUMDB=* bypass sum verification for the luxfi graph.
RUN sed -i '/luxfi\//d' go.sum && go mod download

COPY . .
RUN sed -i '/luxfi\//d' go.sum

# Per SCALE_STANDARD.md §2 (https://github.com/hanzoai/hips/blob/main/docs/SCALE_STANDARD.md)
# — every Go production Dockerfile that emits JSON to a client builds
# with GOEXPERIMENT=jsonv2. Verified -12% time / -23% allocs on the
# edge POST roundtrip vs encoding/json v1.
ARG GO_EXPERIMENT=jsonv2
ENV GOEXPERIMENT=${GO_EXPERIMENT}

# Build zood — the Zoo Network node. The single binary self-installs the Zoo
# EVM (+ AI/FHE/PQ/ZK precompiles, incl. the 0x0303 inference precompile and
# the 0x0304 aivmbridge), Zoo DEX, and Zoo FHE VMs as plugin subprocesses at
# startup (see main.go installPlugin), so no plugin symlinks are needed here.
RUN CGO_ENABLED=1 CGO_CFLAGS="-Wno-incompatible-pointer-types" \
    go build -mod=mod -ldflags="-w -s" -o /build/zood .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/zood /usr/local/bin/zood
# zood dynamically links the luxcpp dex+crypto shared libs (rpath /usr/local/lib)
# and CUDA runtime; carry them from the builder so the runtime can resolve them.
# (No-op layer if the builder base shipped them statically.)
COPY --from=builder /usr/local/lib/ /usr/local/lib/
ENV LD_LIBRARY_PATH=/usr/local/lib
RUN ldconfig 2>/dev/null || true

# Genesis configs baked in for offline bootstrap.
#   genesis.json            — Zoo L2 EVM (chainId 200200)
#   beluga/genesis.json     — Beluga L3 EVM (evmChainId 420420), AI/FHE/PQ/ZK
#   deploy-dex/genesis.json — Zoo DEX
COPY genesis.json /etc/zoo/genesis.json
COPY beluga/genesis.json /etc/zoo/beluga-genesis.json
COPY beluga/config.json /etc/zoo/beluga-config.json
COPY deploy-dex/genesis.json /etc/zoo/dex-genesis.json

ENTRYPOINT ["zood"]
