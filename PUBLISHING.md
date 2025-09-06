# Publishing Zoo Crates to crates.io

This document explains how to publish Zoo crates to crates.io to support both GitHub and crates.io as package sources.

## Prerequisites

1. **Create a crates.io account**: https://crates.io/
2. **Get your API token**: Go to https://crates.io/me and generate an API token
3. **Login to cargo**: Run `cargo login YOUR_API_TOKEN`

## Zoo Crates Structure

The Zoo ecosystem consists of multiple Rust crates that can be published independently:

- `zoo_message_primitives` - Core message types and primitives
- `zoo_crypto_identities` - Cryptographic identity management
- `zoo_tools_primitives` - Tool execution primitives
- `zoo_sqlite` - SQLite database utilities
- `zoo_fs` - Filesystem utilities
- `zoo_embedding` - Embedding generation and management
- `zoo_libp2p_relayer` - P2P networking with libp2p
- `zoo_job_queue_manager` - Job queue management
- `zoo_http_api` - HTTP API utilities
- `zoo_mcp` - Model Context Protocol implementation
- `zoo_tools_runner` - Tool execution runtime (from github.com/zooai/tools)

## Publishing Process

### 1. Update Version Numbers

In the workspace `Cargo.toml`:
```toml
[workspace.package]
version = "1.1.9"  # Increment this
```

### 2. Update Individual Crate Metadata

For each crate in `zoo-libs/`, ensure the `Cargo.toml` has:
```toml
[package]
name = "zoo_your_crate"
version = { workspace = true }
edition = { workspace = true }
authors = { workspace = true }
description = "Brief description of the crate"
license = "MIT OR Apache-2.0"
repository = "https://github.com/zooai/node"
homepage = "https://zoo.ngo"
documentation = "https://docs.zoo.ngo"
keywords = ["zoo", "ai", "your", "keywords", "here"]
categories = ["development-tools"]
```

### 3. Publish Crates

Use the provided script:
```bash
# Make the script executable
chmod +x publish-zoo-crates.sh

# Publish all crates
./publish-zoo-crates.sh

# Or publish a specific crate
./publish-zoo-crates.sh zoo-message-primitives
```

### 4. Using Both GitHub and crates.io

In your `Cargo.toml`, you can now specify dependencies from either source:

#### From crates.io (default):
```toml
zoo_tools_runner = "0.9.7"
```

#### From GitHub (for development):
```toml
zoo_tools_runner = { git = "https://github.com/zooai/node", branch = "main" }
```

#### From local path (for local development):
```toml
zoo_tools_runner = { path = "../zoo-libs/zoo-tools-runner" }
```

## Dual-Source Strategy

The Zoo ecosystem supports both sources:

1. **crates.io**: For stable releases, easier dependency management, and public distribution
2. **GitHub**: For latest development versions, private features, or custom builds

### Benefits:
- **Flexibility**: Users can choose stable (crates.io) or bleeding-edge (GitHub)
- **Redundancy**: If one source is down, the other can be used
- **Development**: Easy to test changes locally or from branches
- **Distribution**: Wider reach through crates.io's ecosystem

## Version Management

Follow semantic versioning:
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

Example:
```
1.0.0 -> 1.0.1 (bug fix)
1.0.1 -> 1.1.0 (new feature)
1.1.0 -> 2.0.0 (breaking change)
```

## Troubleshooting

### "Package already exists" error
- Increment the version number in `Cargo.toml`
- Versions on crates.io are immutable

### "Missing required metadata" error
- Add description, license, and other required fields

### "Dependency not found" error
- Ensure dependencies are published first
- Use the correct order in `publish-zoo-crates.sh`

## Maintenance

Regular maintenance tasks:
1. Keep dependencies up to date
2. Respond to security advisories
3. Update documentation
4. Tag releases in Git: `git tag v1.1.9`
5. Create GitHub releases for major versions

## Contact

For questions about publishing Zoo crates:
- Email: dev@zoo.ngo
- GitHub: https://github.com/zooai/node/issues