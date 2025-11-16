#!/bin/bash

# Simple script to publish remaining zoo crates one by one
set -e

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🚀 Publishing remaining Zoo crates..."
echo "======================================"
echo ""

cd /Users/z/work/zoo/node

# Function to add [workspace] and publish
publish_crate_simple() {
  local crate=$1
  local crate_path="zoo-libs/$crate"
  
  echo ""
  echo "📦 Publishing $crate..."
  
  cd "$crate_path"
  
  # Backup original
  cp Cargo.toml Cargo.toml.backup
  
  # Add empty [workspace] to indicate this is standalone
  echo "" >> Cargo.toml
  echo "[workspace]" >> Cargo.toml
  
  # Publish
  if cargo publish --allow-dirty 2>&1; then
    echo "✅ Successfully published $crate"
  else
    echo "❌ Failed to publish $crate"
  fi
  
  # Restore
  mv Cargo.toml.backup Cargo.toml
  
  cd ../..
  
  # Wait for indexing
  sleep 10
}

# Crates to publish (in dependency order)
# Already published: zoo_message_primitives, zoo_tools_runner, zoo_non_rust_code

# These depend on zoo_non_rust_code (now published)
publish_crate_simple "zoo-fs"
publish_crate_simple "zoo-crypto-identities"

# Independent crates
publish_crate_simple "zoo-mcp"
publish_crate_simple "zoo-embedding"
publish_crate_simple "zoo-baml"

# These depend on other zoo crates
publish_crate_simple "zoo-tools-primitives"  # depends on zoo_mcp
publish_crate_simple "zoo-sqlite"  # depends on zoo_embedding

# Higher level crates
publish_crate_simple "zoo-libp2p-relayer"
publish_crate_simple "zoo-job-queue-manager"
publish_crate_simple "zoo-http-api"

echo ""
echo "✅ Done! Checking published crates..."
cargo search zoo_ --limit 20 | grep "^zoo_" || true