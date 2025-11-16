#!/bin/bash

# Final publishing script with correct dependency order
# Uses Python TOML parser to fix workspace inheritance

set -e
export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"
PYTHON="/opt/homebrew/bin/python3.11"

echo "🚀 Publishing remaining zoo crates in dependency order..."
echo "=========================================================="
echo ""

cd /Users/z/work/zoo/node

# Function to fix and publish a crate
publish_crate() {
  local crate=$1
  echo ""
  echo "📦 Processing $crate..."
  echo "------------------------"

  # Fix workspace inheritance with Python script
  echo "🔧 Fixing workspace inheritance..."
  $PYTHON fix-workspace-toml.py "zoo-libs/$crate"

  # Attempt to publish
  echo "📤 Publishing to crates.io..."
  cd "zoo-libs/$crate"

  if cargo publish --allow-dirty 2>&1; then
    echo "✅ Successfully published $crate"
  else
    echo "❌ Failed to publish $crate (may have missing dependencies)"
  fi

  cd ../..

  # Wait for crates.io to index
  echo "⏳ Waiting 15 seconds for crates.io to index..."
  sleep 15
}

echo "Already published crates:"
echo "- zoo_message_primitives ✅"
echo "- zoo_tools_runner ✅"
echo "- zoo_non_rust_code ✅"
echo "- zoo_crypto_identities ✅"
echo ""

# The remaining crates have circular dependencies so we need to be careful
# Let's start with crates that only depend on already-published crates

echo "🔍 Checking which crates can be published now..."
echo ""

# These should work since they only depend on already published crates:
BATCH1=(
  "zoo-mcp"           # Only depends on zoo_message_primitives
  "zoo-libp2p-relayer" # Only depends on zoo_message_primitives, zoo_crypto_identities
)

echo "BATCH 1: Crates with satisfied dependencies"
echo "============================================"
for crate in "${BATCH1[@]}"; do
  publish_crate "$crate"
done

echo ""
echo "BATCH 1 complete. Checking what we have so far..."
cargo search zoo_ --limit 30 | grep "^zoo_" | sort

# Now we have a problem:
# - zoo-embedding needs zoo_message_primitives (published)
# - zoo-sqlite needs zoo_embedding (not published)
# - zoo-tools-primitives needs zoo_mcp (should be published now), zoo_vector_resources (???)
#
# We need to check if zoo_vector_resources exists or if it's an issue

echo ""
echo "⚠️  WARNING: Some crates have complex circular dependencies"
echo "The following crates reference zoo_vector_resources which doesn't exist as a crate:"
echo "- zoo-tools-primitives"
echo "- zoo-job-queue-manager"
echo ""
echo "We may need to remove or replace these dependencies."
echo ""

# Let's try to publish what we can
BATCH2=(
  "zoo-embedding"  # Should work if it only depends on zoo_message_primitives
)

echo "BATCH 2: zoo-embedding (required by others)"
echo "============================================"
for crate in "${BATCH2[@]}"; do
  publish_crate "$crate"
done

# After zoo-embedding is published, we can try zoo-sqlite
BATCH3=(
  "zoo-sqlite"  # Depends on zoo-embedding
)

echo "BATCH 3: zoo-sqlite (depends on zoo-embedding)"
echo "==============================================="
for crate in "${BATCH3[@]}"; do
  publish_crate "$crate"
done

# After zoo-sqlite is published, we can try zoo-fs
BATCH4=(
  "zoo-fs"  # Depends on zoo-embedding, zoo-sqlite, zoo_non_rust_code
)

echo "BATCH 4: zoo-fs (depends on multiple zoo crates)"
echo "================================================="
for crate in "${BATCH4[@]}"; do
  publish_crate "$crate"
done

# These will likely fail due to zoo_vector_resources
BATCH5=(
  "zoo-tools-primitives"    # Has zoo_vector_resources dependency
  "zoo-job-queue-manager"   # Has zoo_vector_resources dependency
  "zoo-http-api"            # Depends on zoo-tools-primitives
)

echo "BATCH 5: Crates with problematic dependencies"
echo "=============================================="
echo "⚠️  These will likely fail due to zoo_vector_resources"
for crate in "${BATCH5[@]}"; do
  publish_crate "$crate"
done

echo ""
echo "🎉 Publishing attempt complete!"
echo ""
echo "📊 Final check of published crates:"
cargo search zoo_ --limit 30 | grep "^zoo_" | sort
echo ""
echo "✅ Script complete!"