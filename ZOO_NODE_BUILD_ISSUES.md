# Zoo Node Compilation Issues

## Overview
Zoo-node currently has **40 compilation errors** that prevent successful builds. These errors stem from two main categories:

1. **zoo-mcp** (rmcp API compatibility issues) - 3 errors
2. **zoo-tools-primitives** (hanzo_tools_runner integration issues) - 37 errors

Last investigated: 2025-11-23

---

## Issue Category 1: zoo-mcp - RMCP API Version Mismatch

**Affected Crate**: `zoo-libs/zoo-mcp`  
**Root Cause**: zoo-mcp is using outdated rmcp v0.6 API that has breaking changes

### Errors (3 total)

#### Error 1: Missing `SseClientTransport::start` method
```
error[E0599]: no function or associated item named `start` found for struct `SseClientTransport`
 --> zoo-libs/zoo-mcp/src/mcp_methods.rs:50:41
  |
50|     let transport = SseClientTransport::start(sse_url).await.map_err(|e| McpError {
  |                                         ^^^^^ function or associated item not found
```

**Location**: `zoo-libs/zoo-mcp/src/mcp_methods.rs:50`

#### Error 2: Missing fields in `Implementation` struct
```
error[E0063]: missing fields `icons`, `title` and `website_url` in initializer of `Implementation`
 --> zoo-libs/zoo-mcp/src/mcp_methods.rs:56:22
  |
56|         client_info: Implementation {
  |                      ^^^^^^^^^^^^^^ missing `icons`, `title` and `website_url`
```

**Location**: `zoo-libs/zoo-mcp/src/mcp_methods.rs:56`

### Context

The workspace `Cargo.toml` has zoo-mcp **intentionally disabled** with this comment:
```toml
# [dependencies.zoo-mcp]
# workspace = true  # Disabled - zoo-mcp uses outdated rmcp API
```

The workspace also notes in line 15:
```toml
# "zoo-libs/zoo-mcp",  # Keep disabled - uses outdated rmcp API (missing fields, methods)
```

### Recommended Solutions

1. **Update zoo-mcp to rmcp v0.6 API**:
   - Add missing fields (`icons`, `title`, `website_url`) to `Implementation` struct initialization
   - Update `SseClientTransport` usage to match new rmcp v0.6 API
   - Review rmcp v0.6 changelog for other breaking changes

2. **OR Keep zoo-mcp disabled**:
   - Remove zoo-mcp from workspace members permanently
   - Document that zoo-mcp is deprecated/archived
   - Create migration plan if zoo-mcp functionality is needed

---

## Issue Category 2: zoo-tools-primitives - Unresolved `zoo_tools_runner` Imports

**Affected Crate**: `zoo-libs/zoo-tools-primitives`  
**Root Cause**: Cannot resolve `zoo_tools_runner` crate despite correct dependency declaration

### Errors (37 total)

All 37 errors follow the same pattern across multiple files:

```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `zoo_tools_runner`
 --> zoo-libs/zoo-tools-primitives/src/tools/{file}.rs:{line}:{col}
  |
{line} | use zoo_tools_runner::tools::{module}::{Item};
  |     ^^^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `zoo_tools_runner`
```

### Affected Files

**File**: `zoo-libs/zoo-tools-primitives/src/tools/shared_execution.rs`
- Line 9: `use zoo_tools_runner::tools::run_result::RunResult;`

**File**: `zoo-libs/zoo-tools-primitives/src/tools/deno_tools.rs`
- Line 11: `use zoo_tools_runner::tools::code_files::CodeFiles;`
- Line 12: `use zoo_tools_runner::tools::deno_runner::DenoRunner;`
- Line 13: `use zoo_tools_runner::tools::deno_runner_options::DenoRunnerOptions;`
- Line 14: `use zoo_tools_runner::tools::execution_context::ExecutionContext;`
- Line 15+: Multiple additional imports...

### Current Configuration

**`zoo-tools-runner/Cargo.toml`** (FIXED):
```toml
[package]
name = "zoo_tools_runner"
description = "Thin wrapper around hanzo_tools_runner with zoo-specific extensions"

[features]
built-in-tools = ["hanzo_tools_runner/built-in-tools"]

[dependencies]
hanzo_tools_runner = "1.0.3"
serde = { workspace = true, features = ["derive"] }
```

**`zoo-tools-primitives/Cargo.toml`** (FIXED):
```toml
[dependencies.hanzo_tools_runner]
version = "1.0.3"
features = [ "built-in-tools",]

[dependencies.zoo_message_primitives]
version = "1.1.35"
```

**Workspace `Cargo.toml`** (FIXED):
```toml
[workspace.dependencies]
hanzo_tools_runner = "1.0.3"
hanzo_non_rust_code = "1.1.10"
hanzo_message_primitives = "0.0.0"  # Local/vendored dependency

[workspace]
members = [
  "zoo-libs/zoo-tools-primitives",
  "zoo-libs/zoo-tools-runner",
  # ...
]
```

### Problem Analysis

Despite correct dependency declarations:
1. ✅ `zoo_tools_runner` package exists at `zoo-libs/zoo-tools-runner`
2. ✅ `zoo_tools_runner` is listed in workspace members
3. ✅ `hanzo_tools_runner` v1.0.3 is correctly specified
4. ❌ `zoo-tools-primitives` **cannot resolve** `zoo_tools_runner` imports

### Possible Causes

1. **Missing zoo_tools_runner dependency**: `zoo-tools-primitives` declares `hanzo_tools_runner` but tries to import from `zoo_tools_runner`
2. **Missing re-exports**: `zoo_tools_runner` may not be re-exporting the required items from `hanzo_tools_runner`
3. **Incorrect import paths**: Code may be using wrong module paths

### Recommended Solutions

1. **Add zoo_tools_runner dependency to zoo-tools-primitives**:
```toml
# In zoo-libs/zoo-tools-primitives/Cargo.toml
[dependencies]
zoo_tools_runner = { workspace = true }
hanzo_tools_runner = "1.0.3"
```

2. **Ensure zoo_tools_runner re-exports hanzo items**:
```rust
// In zoo-libs/zoo-tools-runner/src/lib.rs
pub use hanzo_tools_runner::tools;
```

3. **OR update import paths in zoo-tools-primitives**:
   Change all:
   ```rust
   use zoo_tools_runner::tools::...
   ```
   To:
   ```rust
   use hanzo_tools_runner::tools::...
   ```

---

## Summary Status

| Category | Errors | Status | Priority |
|----------|--------|--------|----------|
| zoo-mcp rmcp API mismatch | 3 | ⚠️ **Disabled in workspace** | Low |
| zoo-tools-primitives imports | 37 | 🔥 **Blocking builds** | **HIGH** |
| **TOTAL** | **40** | 🚫 **Project does not build** | **CRITICAL** |

---

## Next Steps

### Immediate (High Priority)
1. Fix zoo-tools-primitives import resolution:
   - Add `zoo_tools_runner` to dependencies OR
   - Update all imports to use `hanzo_tools_runner` directly
   - Verify re-exports in `zoo_tools_runner/src/lib.rs`

2. Test build: `cargo build --workspace`

### Future (Low Priority)
3. Decide on zoo-mcp fate:
   - Update to rmcp v0.6 if needed
   - OR permanently archive/remove from workspace

---

## References

- Hanzo tools runner on crates.io: https://crates.io/crates/hanzo_tools_runner (v1.0.3)
- RMCP crate: https://crates.io/crates/rmcp (v0.6)
- Zoo node workspace: `/Users/z/work/zoo/node`
- Related fixes: Commit updating hanzo dependency names from hyphens to underscores (2025-11-23)
