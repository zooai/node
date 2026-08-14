#!/bin/bash

echo "Analyzing zoo crate dependencies..."
echo ""

# Function to extract zoo dependencies from a Cargo.toml
get_zoo_deps() {
    local crate=$1
    echo -n "$crate: "
    grep -E "zoo_[a-z_-]+ =" "zoo-libs/$crate/Cargo.toml" 2>/dev/null | sed 's/.*zoo_/zoo_/' | sed 's/ =.*//' | tr '\n' ' '
    echo ""
}

# Analyze each crate
for crate in zoo-fs zoo-mcp zoo-embedding zoo-tools-primitives zoo-sqlite zoo-libp2p-relayer zoo-job-queue-manager zoo-http-api; do
    if [ -f "zoo-libs/$crate/Cargo.toml" ]; then
        get_zoo_deps "$crate"
    fi
done

echo ""
echo "Dependency Analysis:"
echo "===================="
echo ""
echo "Already published:"
echo "- zoo_message_primitives ✅"
echo "- zoo_non_rust_code ✅"
echo "- zoo_crypto_identities ✅"
echo ""
echo "Suggested publishing order based on dependencies:"
echo ""

# Manually determine order based on the output above
echo "1. zoo-tools-primitives (depends on: zoo_message_primitives)"
echo "2. zoo-embedding (depends on: zoo_message_primitives)"
echo "3. zoo-sqlite (depends on: zoo_embedding)"
echo "4. zoo-fs (depends on: zoo_message_primitives, zoo_embedding, zoo_sqlite, zoo_non_rust_code)"
echo "5. zoo-mcp (depends on: zoo_message_primitives)"
echo "6. zoo-libp2p-relayer (depends on: zoo_message_primitives)"
echo "7. zoo-job-queue-manager (depends on: zoo_message_primitives)"
echo "8. zoo-http-api (depends on: multiple zoo crates)"