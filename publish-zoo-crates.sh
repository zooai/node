#!/bin/bash

# Script to publish Zoo crates to crates.io
# Usage: ./publish-zoo-crates.sh [crate-name]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Zoo crates in dependency order
CRATES=(
    "zoo-message-primitives"
    "zoo-crypto-identities"
    "zoo-tools-primitives"
    "zoo-sqlite"
    "zoo-fs"
    "zoo-embedding"
    "zoo-libp2p-relayer"
    "zoo-job-queue-manager"
    "zoo-http-api"
    "zoo-mcp"
    "zoo-non-rust-code"
)

publish_crate() {
    local crate_path=$1
    local crate_name=$(basename $crate_path)
    
    echo -e "${YELLOW}Publishing $crate_name...${NC}"
    
    cd "$crate_path"
    
    # Verify the crate builds
    cargo build --release
    
    # Run tests
    cargo test
    
    # Dry run first to check for issues
    cargo publish --dry-run
    
    # Publish to crates.io
    echo -e "${GREEN}Publishing $crate_name to crates.io...${NC}"
    cargo publish
    
    # Wait a bit to ensure crates.io indexes the package
    sleep 10
    
    echo -e "${GREEN}✓ $crate_name published successfully${NC}"
}

# Main execution
if [ $# -eq 1 ]; then
    # Publish specific crate
    CRATE_NAME=$1
    CRATE_PATH="zoo-libs/$CRATE_NAME"
    
    if [ -d "$CRATE_PATH" ]; then
        publish_crate "$CRATE_PATH"
    else
        echo -e "${RED}Error: Crate $CRATE_NAME not found${NC}"
        exit 1
    fi
else
    # Publish all crates in order
    echo -e "${YELLOW}Publishing all Zoo crates to crates.io...${NC}"
    
    for crate in "${CRATES[@]}"; do
        CRATE_PATH="zoo-libs/$crate"
        if [ -d "$CRATE_PATH" ]; then
            publish_crate "$CRATE_PATH"
        else
            echo -e "${RED}Warning: Crate $crate not found, skipping...${NC}"
        fi
    done
    
    echo -e "${GREEN}✓ All Zoo crates published successfully!${NC}"
fi