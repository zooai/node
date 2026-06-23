.PHONY: build build-linux test clean docker

BINARY  := zood
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "v0.2.0")
COMMIT  := $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)
LDFLAGS := -ldflags="-w -s -X github.com/luxfi/node/version.GitCommit=$(COMMIT)"

# CGO off by default: the dex's `cgo && linux` path pulls a CUDA bundle, while
# its pure-Go path (and pure-Go secp256k1/sr25519) links everywhere and stays
# deterministic — the only correct mode for a consensus validator. Override with
# `CGO_ENABLED=1 make build` on a box carrying the luxcpp dex+crypto CUDA bundle.
CGO_ENABLED ?= 0
export CGO_ENABLED

build:
	go build -trimpath $(LDFLAGS) -o $(BINARY) .

build-linux:
	GOOS=linux GOARCH=amd64 go build -trimpath $(LDFLAGS) -o $(BINARY)-linux-amd64 .

test:
	go test -v -count=1 ./...

clean:
	rm -f $(BINARY) $(BINARY)-linux-amd64

# Docker build — CI/CD only, never local
docker:
	docker build --platform linux/amd64 -t ghcr.io/zooai/node:latest .
