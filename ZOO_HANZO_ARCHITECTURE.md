# Zoo/Hanzo Architectural Refactoring Plan

## Executive Summary

Zoo should function as a lightweight application layer that leverages Hanzo's robust AI/ML infrastructure rather than duplicating implementations. This approach ensures maintainability, consistency, and reduces technical debt.

## Current Issues

1. **Duplicate Implementations**: Zoo has copies of many Hanzo crates with just namespace changes
2. **Publishing Problems**: Complex workspace inheritance causing crates.io publishing failures
3. **Maintenance Burden**: Need to maintain two parallel codebases
4. **Missing Dependencies**: References to non-existent crates (e.g., zoo_vector_resources)

## Proposed Architecture

### Layer 1: Hanzo Core (Published to crates.io)
These should be published as public crates that Zoo can depend on:

```toml
# Core Infrastructure
hanzo_message_primitives = "1.1.x"
hanzo_crypto_identities = "1.1.x"
hanzo_tools_runner = "1.0.3"  # Already published!

# AI/ML Components  
hanzo_embedding = "1.1.x"
hanzo_hllm = "1.1.x"
hanzo_mining = "1.1.x"
hanzo_mcp = "1.1.x"

# Storage & Networking
hanzo_sqlite = "1.1.x"
hanzo_libp2p_relayer = "1.1.x"
hanzo_fs = "1.1.x"

# API & Tools
hanzo_http_api = "1.1.x"
hanzo_tools_primitives = "1.1.x"
hanzo_job_queue_manager = "1.1.x"
```

### Layer 2: Zoo Extensions (Minimal, Zoo-specific)
Only Zoo-specific features that extend Hanzo:

```toml
# Zoo-specific crates (minimal set)
zoo_message_primitives = { version = "1.1.x" }  # Only if Zoo has unique messages
zoo_crypto_identities = { version = "1.1.x" }   # Only if Zoo has unique identity needs
zoo_mcp = { version = "1.1.x" }                 # Zoo-specific MCP extensions

# Most functionality comes from Hanzo
[dependencies]
hanzo_mining = { version = "1.1.x" }            # Use Hanzo's mining directly
hanzo_embedding = { version = "1.1.x" }         # Use Hanzo's embedding
hanzo_http_api = { version = "1.1.x" }          # Use Hanzo's API
```

## Implementation Steps

### Phase 1: Publish Hanzo Crates (Priority)

1. **Fix and publish core Hanzo crates to crates.io**:
   ```bash
   # Order matters due to dependencies
   1. hanzo_message_primitives
   2. hanzo_crypto_identities  
   3. hanzo_non_rust_code
   4. hanzo_embedding (with reqwest json fix)
   5. hanzo_sqlite
   6. hanzo_fs
   7. hanzo_libp2p_relayer (with reqwest json fix)
   8. hanzo_tools_primitives
   9. hanzo_job_queue_manager
   10. hanzo_http_api
   11. hanzo_mcp (after rmcp fix)
   12. hanzo_mining
   13. hanzo_hllm
   ```

2. **Fixes needed before publishing**:
   - Add `features = ["json"]` to reqwest dependencies
   - Fix rmcp 0.6.4 API breaking changes in hanzo_mcp
   - Ensure all workspace inheritance is resolved

### Phase 2: Refactor Zoo to Use Hanzo

1. **Update zoo-bin/zoo-node/Cargo.toml**:
   ```toml
   [dependencies]
   # Use Hanzo crates from crates.io
   hanzo_mining = "1.1.35"
   hanzo_http_api = "1.1.35"
   hanzo_embedding = "1.1.35"
   hanzo_tools_primitives = "1.1.35"
   
   # Only Zoo-specific crates from local
   zoo_message_primitives = { path = "../../zoo-libs/zoo-message-primitives" }
   zoo_crypto_identities = { path = "../../zoo-libs/zoo-crypto-identities" }
   ```

2. **Update import statements in Rust code**:
   ```rust
   // Before
   use zoo_mining::MiningManager;
   use zoo_http_api::HttpServer;
   
   // After  
   use hanzo_mining::MiningManager;
   use hanzo_http_api::HttpServer;
   ```

3. **Remove duplicate Zoo crates that are just copies**:
   - Delete zoo-libs/zoo-embedding (use hanzo_embedding)
   - Delete zoo-libs/zoo-mining (use hanzo_mining)
   - Delete zoo-libs/zoo-http-api (use hanzo_http_api)
   - Keep only truly Zoo-specific implementations

### Phase 3: Simplify Publishing

1. **Minimal Zoo crates to publish**:
   - zoo_message_primitives (if unique messages)
   - zoo_crypto_identities (if unique identity needs)
   - zoo_non_rust_code (if unique)
   - zoo_mcp (if Zoo-specific MCP extensions)

2. **Dependencies are from crates.io**:
   - All Hanzo dependencies come from crates.io
   - No complex workspace inheritance
   - Simple, clean Cargo.toml files

## Benefits

1. **Reduced Maintenance**: Single source of truth for AI/ML logic
2. **Easier Updates**: Update Hanzo, Zoo gets improvements automatically  
3. **Cleaner Publishing**: Fewer crates to publish, simpler dependencies
4. **Better Architecture**: Clear separation of concerns
5. **Cost Savings**: Less duplicate code to maintain and test

## Example: zoo-tools-primitives Refactoring

### Before (Current)
```toml
[package]
name = "zoo_tools_primitives"

[dependencies]
zoo_message_primitives = { version = "1.1.35" }
zoo_tools_runner = { workspace = true }  # Complex inheritance
zoo_mcp = { version = "1.1.35" }
```

### After (Refactored)
```toml
[package]
name = "zoo_tools_primitives"

[dependencies]
# Use Hanzo's published crates
hanzo_message_primitives = "1.1.35"
hanzo_tools_runner = "1.0.3"  # Already on crates.io!
hanzo_mcp = "1.1.35"

# Only Zoo-specific if needed
zoo_message_primitives = "1.1.35"  # Only if Zoo has unique messages
```

## Migration Checklist

- [ ] Publish hanzo_message_primitives to crates.io
- [ ] Publish hanzo_crypto_identities to crates.io  
- [ ] Publish hanzo_embedding to crates.io (with reqwest fix)
- [ ] Publish hanzo_sqlite to crates.io
- [ ] Publish hanzo_fs to crates.io
- [ ] Publish hanzo_libp2p_relayer to crates.io (with reqwest fix)
- [ ] Publish hanzo_tools_primitives to crates.io
- [ ] Publish hanzo_job_queue_manager to crates.io
- [ ] Publish hanzo_http_api to crates.io
- [ ] Fix and publish hanzo_mcp to crates.io
- [ ] Update zoo-node to use Hanzo crates from crates.io
- [ ] Remove duplicate Zoo crates
- [ ] Publish minimal Zoo-specific crates
- [ ] Update CI/CD pipelines

## Long-term Vision

```
┌─────────────────────────────────────┐
│         Zoo Application             │
│    (Lightweight, Zoo-specific)      │
│                                     │
│  - Zoo branding/UI                 │
│  - Zoo-specific features           │
│  - Zoo network configuration       │
└─────────────────────────────────────┘
                 │
                 │ depends on
                 ▼
┌─────────────────────────────────────┐
│      Hanzo SDK (crates.io)         │
│    (Heavy lifting, AI/ML core)     │
│                                     │
│  - Mining engine                   │
│  - Embedding systems               │
│  - HLLM implementation             │
│  - MCP protocol                    │
│  - HTTP APIs                       │
│  - Storage layers                  │
│  - P2P networking                  │
└─────────────────────────────────────┘
```

This architecture ensures Zoo can focus on its unique value proposition while leveraging Hanzo's robust infrastructure as a foundation.

## Next Steps

1. Get approval for this architectural approach
2. Start publishing Hanzo crates to crates.io (with fixes)
3. Gradually migrate Zoo to use published Hanzo crates
4. Remove duplicate code from Zoo repository
5. Maintain only Zoo-specific extensions

---

*Document created: 2025-11-14*
*Status: Proposed Architecture*