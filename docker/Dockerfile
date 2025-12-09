# ============================================================================
# Zoo Node - Multi-stage Docker Build
# ============================================================================
# This Dockerfile builds the Zoo Node from source with pure environment
# variable configuration. No config files required at runtime.
#
# Build Arguments:
#   BUILD_TYPE: 'release' or 'debug' (default: debug)
#   ZOO_VERSION: Semantic version (e.g., 1.0.0)
#   GLOBAL_IDENTITY_NAME: Node identity (default: @@localhost.sep-zoo)
#
# Runtime Environment Variables:
#   See docker/run_node.sh or .env.example for full list
# ============================================================================

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:bookworm as builder

# Build configuration
ARG BUILD_TYPE=debug
ARG ZOO_VERSION=dev
ARG GLOBAL_IDENTITY_NAME=@@localhost.sep-zoo
ARG BUILD_DATE
ARG VCS_REF

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libclang-dev \
    cmake \
    libssl-dev \
    libc++-dev \
    libc++abi-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy source code
COPY . .

# Add rustfmt for build
RUN rustup component add rustfmt

# Build the project
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    GLOBAL_IDENTITY_NAME="${GLOBAL_IDENTITY_NAME}" cargo build --locked --bin zoo-node $([ "$BUILD_TYPE" = "release" ] && echo "--release") && \
    cp target/${BUILD_TYPE}/zoo-node /build/zoo-node && \
    cp -r target/${BUILD_TYPE}/zoo-tools-runner-resources /build/zoo-tools-runner-resources 2>/dev/null || \
    cp -r target/${BUILD_TYPE}/shinkai-tools-runner-resources /build/zoo-tools-runner-resources 2>/dev/null || true

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim as runner

# Metadata
ARG BUILD_TYPE=debug
ARG ZOO_VERSION=dev
ARG GLOBAL_IDENTITY_NAME=@@localhost.sep-zoo
ARG BUILD_DATE
ARG VCS_REF

LABEL org.opencontainers.image.title="Zoo Node" \
      org.opencontainers.image.description="Zoo AI Network Node" \
      org.opencontainers.image.version="${ZOO_VERSION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.vendor="Zoo AI" \
      org.opencontainers.image.url="https://zoo.ai" \
      org.opencontainers.image.source="https://github.com/zooai/node" \
      org.opencontainers.image.licenses="MIT"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash zoo

# Set working directory
WORKDIR /app

# Copy runtime script
COPY docker/run_node.sh /app/

# Copy built binaries and resources from builder
COPY --from=builder /build/zoo-node /app/
COPY --from=builder /build/zoo-tools-runner-resources /app/zoo-tools-runner-resources

# Make script executable
RUN chmod +x /app/run_node.sh /app/zoo-node

# Set ownership
RUN chown -R zoo:zoo /app

# Switch to non-root user
USER zoo

# Environment variable defaults (can be overridden at runtime)
ENV BUILD_TYPE=${BUILD_TYPE} \
    ZOO_VERSION=${ZOO_VERSION} \
    NODE_API_IP=0.0.0.0 \
    NODE_IP=0.0.0.0 \
    NODE_API_PORT=9550 \
    NODE_WS_PORT=9551 \
    NODE_PORT=9552 \
    NODE_HTTPS_PORT=9553 \
    GLOBAL_IDENTITY_NAME=${GLOBAL_IDENTITY_NAME} \
    RUST_LOG=debug,error,info \
    LOG_SIMPLE=true \
    NO_SECRET_FILE=true \
    STARTING_NUM_QR_PROFILES=1 \
    STARTING_NUM_QR_DEVICES=1 \
    FIRST_DEVICE_NEEDS_REGISTRATION_CODE=false \
    NODE_STORAGE_PATH=zoo-storage

# Expose ports
EXPOSE 9550 9551 9552 9553

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:${NODE_API_PORT}/v2/health_check || exit 1

# Volume for persistent storage
VOLUME ["/app/zoo-storage"]

# Set entrypoint
ENTRYPOINT ["/app/run_node.sh"]
