# syntax=docker/dockerfile:1
# ---------------------------------------------------------------------------
# zood — the Zoo Network node (sovereign L1: Zoo EVM + Zoo DEX + Zoo FHE).
# One image: ghcr.io/zooai/node. CGO_ENABLED=1 (GPU/crypto bridges).
#
# Private module resolution mirrors lux/dex + lux/node: GOPRIVATE for our orgs
# + a gh_token BuildKit secret over HTTPS (luxfi/evm is NOT on the public
# proxy). go.sum is authoritative and verified — there is NO sumdb / go.sum
# bypass (the prior `sed -i /luxfi/d go.sum` + GONOSUMCHECK/GONOSUMDB are gone).
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.6.1 AS xx

FROM --platform=$BUILDPLATFORM golang:1.26-bookworm AS builder
COPY --from=xx / /
RUN apt-get update && apt-get install -y --no-install-recommends \
    git clang lld ca-certificates && rm -rf /var/lib/apt/lists/*
ARG TARGETPLATFORM
RUN xx-apt-get install -y gcc libc6-dev
WORKDIR /build

ENV GOPRIVATE=github.com/luxfi/*,github.com/hanzoai/*,github.com/hanzos3/*,github.com/zooai/*,github.com/parsdao/*,github.com/lux-private/*
ENV GOFLAGS=-mod=mod
ENV GOTOOLCHAIN=auto

COPY go.mod go.sum ./
RUN --mount=type=secret,id=gh_token,required=false \
    if [ -s /run/secrets/gh_token ]; then \
        git config --global url."https://x-access-token:$(cat /run/secrets/gh_token)@github.com/".insteadOf "https://github.com/"; \
    fi && \
    go mod download

COPY . .

# Per SCALE_STANDARD.md §2 — Go services that emit JSON build with jsonv2.
ARG GO_EXPERIMENT=jsonv2
ENV GOEXPERIMENT=${GO_EXPERIMENT}

# zood self-reports its luxfi/node patch level via -X version.VersionPatch
# (otherwise GetVersions() prints the baked default, e.g. 1.30.6).
ARG VERSION_PATCH=73
RUN --mount=type=secret,id=gh_token,required=false \
    if [ -s /run/secrets/gh_token ]; then \
        git config --global url."https://x-access-token:$(cat /run/secrets/gh_token)@github.com/".insteadOf "https://github.com/"; \
    fi && \
    xx-go --wrap && \
    CGO_ENABLED=1 CGO_CFLAGS="-Wno-incompatible-pointer-types" \
    go build -mod=mod \
      -ldflags="-w -s -X github.com/luxfi/node/version.VersionPatch=${VERSION_PATCH}" \
      -o /build/zood .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && rm -rf /var/lib/apt/lists/*
# zood installs its VM plugins (Zoo EVM/DEX/FHE + standard EVM) into the
# runtime --plugin-dir itself at startup (main.go installPlugin), so no
# plugin symlinks are baked here.
RUN mkdir -p /zood/build/plugins
COPY --from=builder /build/zood /zood/build/zood
RUN ln -s /zood/build/zood /usr/local/bin/zood
COPY genesis.json /etc/zoo/genesis.json
ENTRYPOINT ["zood"]
