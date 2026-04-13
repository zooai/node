.PHONY: build build-linux test clean docker

BINARY  := zood
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "v0.2.0")
COMMIT  := $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)
LDFLAGS := -ldflags="-w -s -X github.com/luxfi/node/version.GitCommit=$(COMMIT)"

# CGO required for sr25519-donna + optimized crypto
build:
	CGO_ENABLED=1 go build $(LDFLAGS) -o $(BINARY) .

build-linux:
	CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o $(BINARY)-linux-amd64 .

test:
	go test -v -count=1 ./...

clean:
	rm -f $(BINARY) $(BINARY)-linux-amd64

# Docker build — CI/CD only, never local
docker:
	docker build --platform linux/amd64 -t ghcr.io//node:latest .
