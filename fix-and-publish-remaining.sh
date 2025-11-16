#!/bin/bash

# Fix workspace issues and publish remaining zoo crates
set -e

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🔧 Fixing workspace issues and publishing remaining zoo crates..."
echo "================================================"
echo ""

# List of crates that failed to publish
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

cd /Users/z/work/zoo/node

# Function to add empty workspace and publish
publish_crate_fixed() {
  local crate=$1
  local crate_path="zoo-libs/$crate"

  echo ""
  echo "📦 Processing $crate..."
  echo "------------------------"

  cd "$crate_path"

  # Backup original Cargo.toml
  cp Cargo.toml Cargo.toml.backup

  # Add empty [workspace] table to indicate this is not part of a workspace
  echo "" >> Cargo.toml
  echo "[workspace]" >> Cargo.toml

  echo "✅ Added [workspace] table to $crate/Cargo.toml"

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

# Process each crate
for crate in "${CRATES[@]}"; do
  publish_crate_fixed "$crate"
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
cargo search zoo_ --limit 20 | grep "^zoo"

echo ""
echo "✅ Script complete!"