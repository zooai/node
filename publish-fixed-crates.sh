#!/bin/bash

set -e

echo "🚀 Publishing Fixed Zoo Crates with Code Changes Applied"
echo "======================================================="
echo ""

export CARGO_REGISTRY_TOKEN="cio2uOphUOkgWyPZZQaYFUvFKwpVUBGbyHY"
PYTHON="/opt/homebrew/bin/python3.11"

# Color codes for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track success/failure
SUCCESS_COUNT=0
FAILURE_COUNT=0
FAILED_CRATES=""

echo "📋 Crates to publish:"
echo "  - zoo-embedding (with reqwest json fix)"
echo "  - zoo-libp2p-relayer (with reqwest json fix)"
echo "  - zoo-sqlite (depends on zoo-embedding)"
echo "  - zoo-fs (depends on zoo-embedding, zoo-sqlite)"
echo "  - zoo-tools-primitives"
echo "  - zoo-job-queue-manager (depends on zoo-embedding, zoo-sqlite)"
echo "  - zoo-http-api (depends on zoo-tools-primitives)"
echo ""
echo "Note: Skipping zoo-mcp (needs rmcp API fix)"
echo ""

# Function to publish a crate
publish_crate() {
    local crate=$1
    echo ""
    echo "=================================================================================="
    echo "📦 Publishing $crate"
    echo "=================================================================================="
    
    cd "/Users/z/work/zoo/node/zoo-libs/$crate"
    
    # Fix workspace inheritance using Python script
    echo "🔧 Fixing workspace inheritance..."
    $PYTHON /Users/z/work/zoo/node/fix-workspace-toml.py .
    
    # Try to publish
    echo "📤 Publishing to crates.io..."
    if cargo publish --allow-dirty 2>&1 | tee /tmp/${crate}_publish.log; then
        echo -e "${GREEN}✅ Successfully published $crate${NC}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    else
        echo -e "${RED}❌ Failed to publish $crate${NC}"
        FAILURE_COUNT=$((FAILURE_COUNT + 1))
        FAILED_CRATES="$FAILED_CRATES $crate"
        echo "Error log saved to /tmp/${crate}_publish.log"
    fi
    
    # Small delay to let crates.io index catch up
    echo "⏳ Waiting 10 seconds for crates.io to index..."
    sleep 10
    
    cd - > /dev/null
}

echo "Starting publishing process..."
echo ""

# Publish crates in dependency order
publish_crate "zoo-embedding"
publish_crate "zoo-libp2p-relayer"
publish_crate "zoo-sqlite"
publish_crate "zoo-tools-primitives"
publish_crate "zoo-fs"
publish_crate "zoo-job-queue-manager"
publish_crate "zoo-http-api"

echo ""
echo "=================================================================================="
echo "📊 PUBLISHING SUMMARY"
echo "=================================================================================="
echo -e "${GREEN}✅ Successfully published: $SUCCESS_COUNT crates${NC}"
echo -e "${RED}❌ Failed to publish: $FAILURE_COUNT crates${NC}"

if [ $FAILURE_COUNT -gt 0 ]; then
    echo ""
    echo "Failed crates:$FAILED_CRATES"
    echo ""
    echo "Check error logs in /tmp/ for details"
fi

echo ""
echo "🔍 Checking published crates on crates.io..."
cargo search zoo_ --limit 20 | grep "^zoo_" || true

echo ""
echo "✨ Script complete!"