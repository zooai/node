#!/bin/bash

# Fix dependency versions for publishing to crates.io
# All internal zoo dependencies should use version = "1.1.35"

echo "🔧 Fixing dependency versions in all zoo crates..."

# List of all zoo crates that need fixing
CRATES=(
    "zoo-libs/zoo-crypto-identities"
    "zoo-libs/zoo-fs"
    "zoo-libs/zoo-sqlite"
    "zoo-libs/zoo-tools-primitives"
    "zoo-libs/zoo-libp2p-relayer"
    "zoo-libs/zoo-job-queue-manager"
    "zoo-libs/zoo-embedding"
    "zoo-libs/zoo-http-api"
    "zoo-libs/zoo-tools-runner"
    "zoo-libs/zoo-non-rust-code"
    "zoo-libs/zoo-mcp"
)

for crate_dir in "${CRATES[@]}"; do
    CARGO_FILE="$crate_dir/Cargo.toml"

    if [ -f "$CARGO_FILE" ]; then
        echo "📝 Fixing $crate_dir..."

        # Replace workspace dependencies with explicit versions
        sed -i '' 's/zoo_message_primitives = { workspace = true }/zoo_message_primitives = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_crypto_identities = { workspace = true }/zoo_crypto_identities = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_fs = { workspace = true }/zoo_fs = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_sqlite = { workspace = true }/zoo_sqlite = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_tools_primitives = { workspace = true }/zoo_tools_primitives = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_libp2p_relayer = { workspace = true }/zoo_libp2p_relayer = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_job_queue_manager = { workspace = true }/zoo_job_queue_manager = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_embedding = { workspace = true }/zoo_embedding = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_http_api = { workspace = true }/zoo_http_api = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_tools_runner = { workspace = true }/zoo_tools_runner = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_non_rust_code = { workspace = true }/zoo_non_rust_code = { version = "1.1.35" }/' "$CARGO_FILE"
        sed -i '' 's/zoo_mcp = { workspace = true }/zoo_mcp = { version = "1.1.35" }/' "$CARGO_FILE"

        # Also fix hanzo dependencies to use published versions
        sed -i '' 's/hanzo_tools_runner = { workspace = true }/hanzo_tools_runner = { version = "1.0.3" }/' "$CARGO_FILE"
        sed -i '' 's/hanzo_non_rust_code = { workspace = true }/hanzo_non_rust_code = { path = "..\/..\/vendor\/hanzo-non-rust-code" }/' "$CARGO_FILE"
        sed -i '' 's/hanzo_message_primitives = { workspace = true }/hanzo_message_primitives = { path = "..\/..\/vendor\/hanzo-message-primitives" }/' "$CARGO_FILE"
    fi
done

echo ""
echo "✅ All Cargo.toml files updated with explicit version numbers!"
echo ""
echo "Next step: Run publish-crates.sh to publish all crates"