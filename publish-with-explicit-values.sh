#!/bin/bash

# Publish remaining zoo crates with explicit values instead of workspace inheritance
set -e

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🔧 Publishing remaining zoo crates with explicit values..."
echo "================================================"
echo ""

cd /Users/z/work/zoo/node

# Function to replace workspace inheritance and publish
publish_crate_explicit() {
  local crate=$1
  local crate_path="zoo-libs/$crate"

  echo ""
  echo "📦 Processing $crate..."
  echo "------------------------"

  cd "$crate_path"

  # Backup original Cargo.toml
  cp Cargo.toml Cargo.toml.backup

  # Replace all workspace = true with explicit values
  sed -i '' 's/version = { workspace = true }/version = "1.1.35"/' Cargo.toml
  sed -i '' 's/edition = { workspace = true }/edition = "2021"/' Cargo.toml
  sed -i '' 's/authors = { workspace = true }/authors = ["Zoo Foundation <dev@zoo.ngo>"]/' Cargo.toml
  sed -i '' 's/license = { workspace = true }/license = "MIT"/' Cargo.toml
  sed -i '' 's/repository = { workspace = true }/repository = "https:\/\/github.com\/zooai\/node"/' Cargo.toml
  sed -i '' 's/homepage = { workspace = true }/homepage = "https:\/\/zoo.ai"/' Cargo.toml

  echo "✅ Replaced workspace inheritance with explicit values"

  # Try to publish
  echo "📤 Publishing $crate to crates.io..."
  if cargo publish --allow-dirty 2>&1; then
    echo "✅ Successfully published $crate"
    PUBLISHED+=("$crate")
  else
    echo "❌ Failed to publish $crate"
    FAILED+=("$crate")
  fi

  # Restore original Cargo.toml
  mv Cargo.toml.backup Cargo.toml
  echo "✅ Restored original Cargo.toml for $crate"

  # Wait for crates.io to index
  echo "⏳ Waiting 10 seconds for crates.io to index..."
  sleep 10

  cd ../..
}

# Arrays to track results
PUBLISHED=()
FAILED=()

# List of crates that need to be published
CRATES=(
  "zoo-sqlite"
  "zoo-tools-primitives"
  "zoo-non-rust-code"
  "zoo-fs"
  "zoo-crypto-identities"
  "zoo-libp2p-relayer"
  "zoo-job-queue-manager"
  "zoo-embedding"
  "zoo-http-api"
  "zoo-mcp"
)

# Process each crate
for crate in "${CRATES[@]}"; do
  publish_crate_explicit "$crate"
done

echo ""
echo "================================================"
echo "📊 Publishing Results"
echo "================================================"
echo ""

if [ ${#PUBLISHED[@]} -gt 0 ]; then
  echo "✅ Successfully published (${#PUBLISHED[@]} crates):"
  for crate in "${PUBLISHED[@]}"; do
    echo "   - $crate"
  done
fi

if [ ${#FAILED[@]} -gt 0 ]; then
  echo ""
  echo "❌ Failed to publish (${#FAILED[@]} crates):"
  for crate in "${FAILED[@]}"; do
    echo "   - $crate"
  done
fi

echo ""
echo "🔍 Verifying published crates on crates.io..."
echo ""
cargo search zoo_ --limit 20 | grep "^zoo" || true
echo ""
cargo search zoo- --limit 20 | grep "^zoo" || true

echo ""
echo "✅ Script complete!"