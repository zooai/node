#!/bin/bash

# Publishing remaining zoo crates
set -e

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"

echo "🚀 Publishing remaining zoo crates..."
echo ""

# Crates to publish (in dependency order)
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

for crate in "${CRATES[@]}"; do
  echo "📦 Publishing $crate..."
  cd "/Users/z/work/zoo/node/zoo-libs/$crate"

  # Try to publish
  if cargo publish --allow-dirty 2>&1 | tee /tmp/publish-$crate.log | tail -5; then
    echo "✅ Successfully published $crate"
  else
    echo "❌ Failed to publish $crate (check /tmp/publish-$crate.log)"
  fi

  echo "⏳ Waiting 10 seconds for crates.io to index..."
  sleep 10
  echo ""
done

echo "🎉 Publishing complete!"
echo ""
echo "📊 Summary of published crates:"
cargo search zoo_ --limit 20 | grep "^zoo"