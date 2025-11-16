#!/bin/bash

# Final script to publish all remaining zoo crates
set -e

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🚀 Publishing remaining Zoo crates (final attempt)..."
echo "===================================================="
echo ""

cd /Users/z/work/zoo/node

# Function to publish with workspace inheritance replacement
publish_crate_final() {
  local crate=$1
  local crate_path="zoo-libs/$crate"
  
  echo ""
  echo "📦 Publishing $crate..."
  
  cd "$crate_path"
  
  # Backup
  cp Cargo.toml Cargo.toml.backup
  
  # Replace ALL workspace inheritance with explicit values
  sed -i '' 's/version = { workspace = true }/version = "1.1.35"/' Cargo.toml
  sed -i '' 's/edition = { workspace = true }/edition = "2021"/' Cargo.toml
  sed -i '' 's/authors = { workspace = true }/authors = ["Zoo Foundation <dev@zoo.ngo>"]/' Cargo.toml
  sed -i '' 's/license = { workspace = true }/license = "MIT"/' Cargo.toml
  sed -i '' 's/repository = { workspace = true }/repository = "https:\/\/github.com\/zooai\/node"/' Cargo.toml
  sed -i '' 's/homepage = { workspace = true }/homepage = "https:\/\/zoo.ai"/' Cargo.toml
  sed -i '' 's/keywords = { workspace = true }/keywords = ["ai", "blockchain", "decentralized", "llm"]/' Cargo.toml
  sed -i '' 's/categories = { workspace = true }/categories = ["cryptography", "network-programming"]/' Cargo.toml
  
  # Add [workspace] to indicate standalone
  echo "" >> Cargo.toml
  echo "[workspace]" >> Cargo.toml
  
  # Try to publish
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

# Crates that depend on zoo_non_rust_code (now published)
echo "=== Publishing crates depending on zoo_non_rust_code ==="
publish_crate_final "zoo-fs"
publish_crate_final "zoo-crypto-identities"

# Independent crates
echo ""
echo "=== Publishing independent crates ==="
publish_crate_final "zoo-mcp"
publish_crate_final "zoo-embedding"
publish_crate_final "zoo-baml"

# Crates with dependencies
echo ""
echo "=== Publishing crates with zoo dependencies ==="
publish_crate_final "zoo-tools-primitives"
publish_crate_final "zoo-sqlite"

# Higher level crates
echo ""
echo "=== Publishing higher level crates ==="
publish_crate_final "zoo-libp2p-relayer"
publish_crate_final "zoo-job-queue-manager"
publish_crate_final "zoo-http-api"

echo ""
echo "=== Final verification ==="
cargo search zoo_ --limit 30 | grep "^zoo_" | sort