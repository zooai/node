#!/bin/bash

# Zoo Crates Publishing Script - Correct Dependency Order
# This script publishes all zoo crates to crates.io in the correct dependency order

set -e  # Exit on error

# Check if token is set
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    echo "❌ Error: CARGO_REGISTRY_TOKEN not set"
    echo "Please run: export CARGO_REGISTRY_TOKEN=your_token"
    exit 1
fi

echo "🚀 Starting Zoo Crates Publishing to crates.io"
echo "==========================================="

# Function to publish a crate
publish_crate() {
    local crate_path=$1
    local crate_name=$(basename $crate_path)

    echo ""
    echo "📦 Publishing $crate_name..."

    cd "$crate_path"

    # Check if already published
    if cargo search "$crate_name" | grep -q "^$crate_name "; then
        echo "⚠️  $crate_name already published, checking version..."

        # Get local version
        local local_version=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2 | cut -d' ' -f3)

        # Get published version
        local published_version=$(cargo search "$crate_name" --limit 1 | head -1 | awk '{print $3}' | tr -d '"')

        if [ "$local_version" = "$published_version" ]; then
            echo "✅ $crate_name v$local_version already published, skipping"
            return 0
        else
            echo "📤 Publishing new version: v$local_version (current: v$published_version)"
        fi
    fi

    # Dry run first
    echo "🔍 Dry run for $crate_name..."
    cargo publish --dry-run --allow-dirty

    # Actual publish
    echo "📤 Publishing $crate_name to crates.io..."
    cargo publish --allow-dirty

    echo "✅ Successfully published $crate_name"

    # Wait a bit for crates.io to index
    echo "⏳ Waiting for crates.io to index..."
    sleep 10
}

# Navigate to zoo node directory
cd /Users/z/work/zoo/node

echo "📋 Publishing order (based on dependencies):"
echo ""
echo "Group 1: Base crates with no zoo dependencies"
echo "  1. zoo-message-primitives (already published v1.1.35)"
echo "  2. zoo-tools-runner (depends only on hanzo_tools_runner v1.0.3)"
echo ""
echo "Group 2: Crates that depend on zoo-message-primitives and/or zoo-tools-runner"
echo "  3. zoo-tools-primitives"
echo "  4. zoo-sqlite"
echo "  5. zoo-non-rust-code (depends on zoo-tools-runner)"
echo ""
echo "Group 3: Crates that depend on Group 2 crates"
echo "  6. zoo-crypto-identities (depends on zoo-non-rust-code)"
echo "  7. zoo-fs (depends on zoo-non-rust-code)"
echo ""
echo "Group 4: Higher-level crates"
echo "  8. zoo-libp2p-relayer"
echo "  9. zoo-job-queue-manager"
echo "  10. zoo-embedding"
echo "  11. zoo-http-api"
echo ""
echo "Group 5: Final crate"
echo "  12. zoo-mcp"
echo ""
echo "==========================================="

# Group 1: Base crates (no zoo dependencies)
echo ""
echo "📦 GROUP 1: Base crates"
echo "----------------------"
# zoo-message-primitives already published
echo "✅ zoo-message-primitives v1.1.35 already published"
publish_crate "zoo-libs/zoo-tools-runner"

# Group 2: Crates that depend on base crates
echo ""
echo "📦 GROUP 2: First-level dependencies"
echo "------------------------------------"
publish_crate "zoo-libs/zoo-tools-primitives"
publish_crate "zoo-libs/zoo-sqlite"
publish_crate "zoo-libs/zoo-non-rust-code"

# Group 3: Crates that depend on Group 2
echo ""
echo "📦 GROUP 3: Second-level dependencies"
echo "-------------------------------------"
publish_crate "zoo-libs/zoo-crypto-identities"
publish_crate "zoo-libs/zoo-fs"

# Group 4: Higher-level crates
echo ""
echo "📦 GROUP 4: Higher-level crates"
echo "--------------------------------"
publish_crate "zoo-libs/zoo-libp2p-relayer"
publish_crate "zoo-libs/zoo-job-queue-manager"
publish_crate "zoo-libs/zoo-embedding"
publish_crate "zoo-libs/zoo-http-api"

# Group 5: Final crate
echo ""
echo "📦 GROUP 5: Final crate"
echo "-----------------------"
publish_crate "zoo-libs/zoo-mcp"

echo ""
echo "🎉 All zoo crates published successfully!"
echo ""
echo "📊 Summary:"
cargo search zoo_ --limit 20 | grep "^zoo"
echo ""
echo "✅ Next steps:"
echo "1. Update zoo-node's Cargo.toml to use published versions instead of path dependencies"
echo "2. Remove the vendor directory"
echo "3. Push the changes to trigger a new CI build"
echo "4. Monitor CI build to confirm all binaries build successfully"