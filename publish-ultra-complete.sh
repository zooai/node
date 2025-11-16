#!/bin/bash

# Ultra-complete fix for publishing remaining zoo crates
# This script replaces ALL workspace inheritance patterns

set -e
export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🚀 Publishing remaining zoo crates with ULTRA-COMPLETE workspace fix..."
echo "=============================================================="
echo ""

cd /Users/z/work/zoo/node

# Function to completely fix and publish a crate
publish_crate_ultra() {
  local crate=$1
  local crate_path="zoo-libs/$crate"

  echo ""
  echo "📦 Processing $crate..."
  echo "------------------------"

  if [ ! -d "$crate_path" ]; then
    echo "⚠️ Directory $crate_path not found, skipping..."
    return
  fi

  cd "$crate_path"

  # Backup original
  cp Cargo.toml Cargo.toml.original

  # Step 1: Replace package metadata
  sed -i '' 's/version = { workspace = true }/version = "1.1.35"/' Cargo.toml
  sed -i '' 's/edition = { workspace = true }/edition = "2021"/' Cargo.toml
  sed -i '' 's/authors = { workspace = true }/authors = ["Zoo Foundation <dev@zoo.ngo>"]/' Cargo.toml
  sed -i '' 's/license = { workspace = true }/license = "MIT"/' Cargo.toml
  sed -i '' 's/repository = { workspace = true }/repository = "https:\/\/github.com\/zooai\/node"/' Cargo.toml
  sed -i '' 's/homepage = { workspace = true }/homepage = "https:\/\/zoo.ai"/' Cargo.toml

  # Step 2: Replace ALL dependency workspace inheritance (including features)
  # Handle both simple and complex forms

  # Simple forms: dependency = { workspace = true }
  sed -i '' 's/futures = { workspace = true }/futures = "0.3.30"/' Cargo.toml
  sed -i '' 's/tokio = { workspace = true }/tokio = { version = "1.36", features = ["rt", "rt-multi-thread", "macros", "fs", "io-util", "net", "sync", "time"] }/' Cargo.toml
  sed -i '' 's/tokio-util = { workspace = true }/tokio-util = "0.7.13"/' Cargo.toml
  sed -i '' 's/bincode = { workspace = true }/bincode = "1.3.3"/' Cargo.toml
  sed -i '' 's/log = { workspace = true }/log = "0.4.20"/' Cargo.toml
  sed -i '' 's/chrono = { workspace = true }/chrono = "0.4"/' Cargo.toml
  sed -i '' 's/serde_json = { workspace = true }/serde_json = "1.0.117"/' Cargo.toml
  sed -i '' 's/anyhow = { workspace = true }/anyhow = "1.0.94"/' Cargo.toml
  sed -i '' 's/blake3 = { workspace = true }/blake3 = "1.2.0"/' Cargo.toml
  sed -i '' 's/serde = { workspace = true }/serde = { version = "1.0.219", features = ["derive"] }/' Cargo.toml
  sed -i '' 's/base64 = { workspace = true }/base64 = "0.22.0"/' Cargo.toml
  sed -i '' 's/reqwest = { workspace = true }/reqwest = "0.11.27"/' Cargo.toml
  sed -i '' 's/regex = { workspace = true }/regex = "1"/' Cargo.toml
  sed -i '' 's/uuid = { workspace = true }/uuid = { version = "1.6.1" }/' Cargo.toml
  sed -i '' 's/rand = { workspace = true }/rand = "0.8.5"/' Cargo.toml
  sed -i '' 's/hex = { workspace = true }/hex = "0.4.3"/' Cargo.toml
  sed -i '' 's/env_logger = { workspace = true }/env_logger = "0.11.5"/' Cargo.toml
  sed -i '' 's/async-trait = { workspace = true }/async-trait = "0.1.74"/' Cargo.toml
  sed -i '' 's/ed25519-dalek = { workspace = true }/ed25519-dalek = { version = "2.1.1", features = ["rand_core"] }/' Cargo.toml
  sed -i '' 's/x25519-dalek = { workspace = true }/x25519-dalek = { version = "2.0.1", features = ["static_secrets"] }/' Cargo.toml
  sed -i '' 's/tempfile = { workspace = true }/tempfile = "3.19"/' Cargo.toml
  sed -i '' 's/lazy_static = { workspace = true }/lazy_static = "1.5.0"/' Cargo.toml
  sed -i '' 's/async-channel = { workspace = true }/async-channel = "1.6.1"/' Cargo.toml
  sed -i '' 's/csv = { workspace = true }/csv = "1.1.6"/' Cargo.toml
  sed -i '' 's/thiserror = { workspace = true }/thiserror = "2.0.3"/' Cargo.toml
  sed -i '' 's/dashmap = { workspace = true }/dashmap = "5.5.3"/' Cargo.toml
  sed -i '' 's/clap = { workspace = true }/clap = "3.0.0-beta.5"/' Cargo.toml
  sed -i '' 's/r2d2 = { workspace = true }/r2d2 = "0.8.10"/' Cargo.toml
  sed -i '' 's/r2d2_sqlite = { workspace = true }/r2d2_sqlite = "0.25"/' Cargo.toml
  sed -i '' 's/rusqlite = { workspace = true }/rusqlite = { version = "0.32.1", features = ["bundled"] }/' Cargo.toml
  sed -i '' 's/os_path = { workspace = true }/os_path = "0.8.0"/' Cargo.toml
  sed -i '' 's/utoipa = { workspace = true }/utoipa = "4.2.3"/' Cargo.toml
  sed -i '' 's/warp = { workspace = true }/warp = "0.3.7"/' Cargo.toml
  sed -i '' 's/once_cell = { workspace = true }/once_cell = "1.21"/' Cargo.toml
  sed -i '' 's/home = { workspace = true }/home = "0.5"/' Cargo.toml
  sed -i '' 's/strip-ansi-escapes = { workspace = true }/strip-ansi-escapes = "0.2"/' Cargo.toml
  sed -i '' 's/tracing = { workspace = true }/tracing = "0.1.40"/' Cargo.toml
  sed -i '' 's/serde_yaml = { workspace = true }/serde_yaml = "0.9.34-deprecated"/' Cargo.toml
  sed -i '' 's/tokio-tungstenite = { workspace = true }/tokio-tungstenite = "0.26.2"/' Cargo.toml
  sed -i '' 's/rustls = { workspace = true }/rustls = "0.23.27"/' Cargo.toml
  sed -i '' 's/libp2p = { workspace = true }/libp2p = { version = "0.55.0", features = ["noise", "yamux", "tcp", "quic", "dcutr", "identify", "ping", "relay", "request-response", "json", "tokio", "macros"] }/' Cargo.toml
  sed -i '' 's/rmcp = { workspace = true }/rmcp = { version = "0.6" }/' Cargo.toml
  sed -i '' 's/keyphrases = { workspace = true }/keyphrases = "0.3.3"/' Cargo.toml

  # Handle complex forms with features already specified but workspace = true
  # Pattern: dependency = { workspace = true, features = [...] }
  perl -i -pe 's/serde = \{ workspace = true, features = \[([^\]]+)\] \}/serde = { version = "1.0.219", features = [$1] }/g' Cargo.toml
  perl -i -pe 's/tokio = \{ workspace = true, features = \[([^\]]+)\] \}/tokio = { version = "1.36", features = [$1] }/g' Cargo.toml
  perl -i -pe 's/reqwest = \{ workspace = true, features = \[([^\]]+)\] \}/reqwest = { version = "0.11.27", features = [$1] }/g' Cargo.toml
  perl -i -pe 's/uuid = \{ workspace = true, features = \[([^\]]+)\] \}/uuid = { version = "1.6.1", features = [$1] }/g' Cargo.toml
  perl -i -pe 's/chrono = \{ workspace = true, features = \[([^\]]+)\] \}/chrono = { version = "0.4", features = [$1] }/g' Cargo.toml
  perl -i -pe 's/clap = \{ workspace = true, features = \[([^\]]+)\] \}/clap = { version = "3.0.0-beta.5", features = [$1] }/g' Cargo.toml

  # Step 3: Fix internal zoo dependencies to use published versions
  sed -i '' 's/zoo_message_primitives = { path = "[^"]*" }/zoo_message_primitives = "1.1.35"/' Cargo.toml
  sed -i '' 's/zoo_tools_runner = { path = "[^"]*" }/zoo_tools_runner = "1.1.35"/' Cargo.toml
  sed -i '' 's/zoo_non_rust_code = { path = "[^"]*" }/zoo_non_rust_code = "1.1.35"/' Cargo.toml
  sed -i '' 's/zoo_crypto_identities = { path = "[^"]*" }/zoo_crypto_identities = "1.1.35"/' Cargo.toml

  # Step 4: Add empty [workspace] table at the end
  echo "" >> Cargo.toml
  echo "[workspace]" >> Cargo.toml

  echo "✅ Fixed all workspace inheritance for $crate"

  # Step 5: Try to publish
  echo "📤 Publishing $crate to crates.io..."
  if cargo publish --allow-dirty 2>&1 | tee publish.log; then
    echo "✅ Successfully published $crate"
  else
    echo "❌ Failed to publish $crate"
    echo "Error details:"
    grep -E "error|failed|Caused by" publish.log || cat publish.log
  fi

  # Restore original
  mv Cargo.toml.original Cargo.toml

  # Wait for crates.io to index
  echo "⏳ Waiting 10 seconds for crates.io to index..."
  sleep 10

  cd ../..
}

# Crates to publish (skip already published ones)
CRATES=(
  "zoo-fs"
  "zoo-mcp"
  "zoo-embedding"
  "zoo-tools-primitives"
  "zoo-sqlite"
  "zoo-libp2p-relayer"
  "zoo-job-queue-manager"
  "zoo-http-api"
)

# Process each crate
for crate in "${CRATES[@]}"; do
  publish_crate_ultra "$crate"
done

echo ""
echo "🎉 Publishing complete!"
echo ""
echo "📊 Verifying published crates..."
cargo search zoo_ --limit 30 | grep "^zoo_" | sort

echo ""
echo "✅ Script complete!"