#!/usr/bin/env python3
"""
Fix workspace inheritance in Cargo.toml files for publishing to crates.io.
This script properly handles complex TOML structures that sed/perl cannot.
"""

import toml
import sys
import os
from pathlib import Path
from typing import Dict, Any

# Workspace dependency versions from root Cargo.toml
WORKSPACE_DEPS = {
    "futures": "0.3.30",
    "tokio": {"version": "1.36", "features": ["rt", "rt-multi-thread", "macros", "fs", "io-util", "net", "sync", "time"]},
    "tokio-util": "0.7.13",
    "bincode": "1.3.3",
    "log": "0.4.20",
    "chrono": "0.4",
    "serde_json": "1.0.117",
    "anyhow": "1.0.94",
    "blake3": "1.2.0",
    "serde": {"version": "1.0.219", "features": ["derive"]},
    "base64": "0.22.0",
    "reqwest": "0.11.27",
    "regex": "1",
    "uuid": {"version": "1.6.1"},
    "rand": "0.8.5",
    "hex": "0.4.3",
    "env_logger": "0.11.5",
    "async-trait": "0.1.74",
    "ed25519-dalek": {"version": "2.1.1", "features": ["rand_core"]},
    "x25519-dalek": {"version": "2.0.1", "features": ["static_secrets"]},
    "tempfile": "3.19",
    "lazy_static": "1.5.0",
    "async-channel": "1.6.1",
    "csv": "1.1.6",
    "thiserror": "2.0.3",
    "dashmap": "5.5.3",
    "clap": "3.0.0-beta.5",
    "r2d2": "0.8.10",
    "r2d2_sqlite": "0.25",
    "rusqlite": {"version": "0.32.1", "features": ["bundled"]},
    "os_path": "0.8.0",
    "utoipa": "4.2.3",
    "warp": "0.3.7",
    "once_cell": "1.21",
    "home": "0.5",
    "strip-ansi-escapes": "0.2",
    "tracing": "0.1.40",
    "serde_yaml": "0.9.34-deprecated",
    "tokio-tungstenite": "0.26.2",
    "rustls": "0.23.27",
    "libp2p": {"version": "0.55.0", "features": ["noise", "yamux", "tcp", "quic", "dcutr", "identify", "ping", "relay", "request-response", "json", "tokio", "macros"]},
    "rmcp": {"version": "0.6"},
    "keyphrases": "0.3.3",
}

# Package metadata values
PACKAGE_META = {
    "version": "1.1.35",
    "edition": "2021",
    "authors": ["Zoo Foundation <dev@zoo.ngo>"],
    "license": "MIT",
    "repository": "https://github.com/zooai/node",
    "homepage": "https://zoo.ai",
}


def fix_package_section(data: Dict[str, Any]) -> None:
    """Fix the [package] section by replacing workspace inheritance."""
    if "package" not in data:
        return

    package = data["package"]
    for key, value in package.items():
        if isinstance(value, dict) and value.get("workspace") is True:
            if key in PACKAGE_META:
                package[key] = PACKAGE_META[key]
            else:
                print(f"Warning: Unknown package field: {key}")


def resolve_dependency(dep_name: str, dep_value: Any) -> Any:
    """Resolve a single dependency from workspace inheritance."""
    # If it's already a string (version), return as-is
    if isinstance(dep_value, str):
        return dep_value

    # If it's a dict with workspace = true
    if isinstance(dep_value, dict):
        if dep_value.get("workspace") is True:
            # Check if we have a workspace definition
            if dep_name in WORKSPACE_DEPS:
                workspace_def = WORKSPACE_DEPS[dep_name]
                # If workspace def is a string, simple replacement
                if isinstance(workspace_def, str):
                    return workspace_def
                # If workspace def is a dict, merge with any additional features
                elif isinstance(workspace_def, dict):
                    result = workspace_def.copy()
                    # Preserve any additional keys from the original (like extra features)
                    for key, val in dep_value.items():
                        if key != "workspace" and key not in result:
                            result[key] = val
                        # Special handling for features - merge them
                        elif key == "features" and "features" in result:
                            existing = set(result["features"])
                            additional = set(val) if isinstance(val, list) else {val}
                            result["features"] = list(existing | additional)
                    return result
            else:
                print(f"Warning: No workspace definition for {dep_name}")
                return dep_value
        # If it has version/path but not workspace, keep as-is
        elif "version" in dep_value or "path" in dep_value:
            return dep_value
        # If it has workspace = false or no workspace key, keep as-is
        else:
            return dep_value

    return dep_value


def fix_dependencies_section(data: Dict[str, Any], section_name: str) -> None:
    """Fix a dependencies section by resolving workspace inheritance."""
    if section_name not in data:
        return

    deps = data[section_name]
    for dep_name, dep_value in list(deps.items()):
        resolved = resolve_dependency(dep_name, dep_value)
        deps[dep_name] = resolved


def fix_dependencies_table_syntax(data: Dict[str, Any]) -> None:
    """Handle [dependencies.package_name] table syntax."""
    # Look for keys like "dependencies.serde"
    for key in list(data.keys()):
        if key.startswith("dependencies.") or key.startswith("dev-dependencies."):
            parts = key.split(".", 1)
            section = parts[0]
            dep_name = parts[1]

            # Ensure the main section exists
            if section not in data:
                data[section] = {}

            # Move the table content to the main section
            table_content = data[key]
            if isinstance(table_content, dict) and table_content.get("workspace") is True:
                # Resolve from workspace
                if dep_name in WORKSPACE_DEPS:
                    workspace_def = WORKSPACE_DEPS[dep_name]
                    if isinstance(workspace_def, str):
                        resolved = {"version": workspace_def}
                    else:
                        resolved = workspace_def.copy()

                    # Merge with any additional fields
                    for field, value in table_content.items():
                        if field != "workspace":
                            if field == "features" and "features" in resolved:
                                # Merge features
                                existing = set(resolved["features"])
                                additional = set(value) if isinstance(value, list) else {value}
                                resolved["features"] = list(existing | additional)
                            elif field not in resolved:
                                resolved[field] = value

                    data[section][dep_name] = resolved
                else:
                    # Keep as-is if no workspace def found
                    data[section][dep_name] = table_content
            else:
                # Not workspace inherited, just move it
                data[section][dep_name] = table_content

            # Remove the table syntax key
            del data[key]


def fix_zoo_dependencies(data: Dict[str, Any]) -> None:
    """Fix internal zoo dependencies to use published versions."""
    sections = ["dependencies", "dev-dependencies", "build-dependencies"]

    for section in sections:
        if section not in data:
            continue

        deps = data[section]
        for dep_name in list(deps.keys()):
            if dep_name.startswith("zoo_"):
                dep_value = deps[dep_name]
                # If it's a path dependency, convert to version
                if isinstance(dep_value, dict) and "path" in dep_value:
                    # Use the published version
                    deps[dep_name] = {"version": "1.1.35"}
                elif isinstance(dep_value, dict) and "version" not in dep_value:
                    # Add version if missing
                    dep_value["version"] = "1.1.35"


def add_workspace_table(data: Dict[str, Any]) -> None:
    """Add an empty [workspace] table to indicate standalone crate."""
    if "workspace" not in data:
        data["workspace"] = {}


def process_cargo_toml(file_path: Path) -> bool:
    """Process a single Cargo.toml file."""
    print(f"Processing {file_path}")

    try:
        # Read the TOML file
        with open(file_path, 'r') as f:
            content = f.read()

        # Parse TOML
        data = toml.loads(content)

        # Apply fixes
        fix_package_section(data)
        fix_dependencies_table_syntax(data)
        fix_dependencies_section(data, "dependencies")
        fix_dependencies_section(data, "dev-dependencies")
        fix_dependencies_section(data, "build-dependencies")
        fix_zoo_dependencies(data)
        add_workspace_table(data)

        # Write back
        with open(file_path, 'w') as f:
            toml.dump(data, f)

        print(f"✅ Successfully fixed {file_path}")
        return True

    except Exception as e:
        print(f"❌ Error processing {file_path}: {e}")
        return False


def main():
    """Main function to fix a specific crate or all crates."""
    if len(sys.argv) > 1:
        # Process specific crate
        crate_path = Path(sys.argv[1])
        if crate_path.is_dir():
            cargo_toml = crate_path / "Cargo.toml"
            if cargo_toml.exists():
                success = process_cargo_toml(cargo_toml)
                sys.exit(0 if success else 1)
            else:
                print(f"Error: No Cargo.toml found in {crate_path}")
                sys.exit(1)
        else:
            print(f"Error: {crate_path} is not a directory")
            sys.exit(1)
    else:
        # Process all zoo-libs crates
        zoo_libs = Path("zoo-libs")
        if not zoo_libs.exists():
            print(f"Error: zoo-libs directory not found")
            sys.exit(1)

        crates = [
            "zoo-fs",
            "zoo-mcp",
            "zoo-embedding",
            "zoo-tools-primitives",
            "zoo-sqlite",
            "zoo-libp2p-relayer",
            "zoo-job-queue-manager",
            "zoo-http-api",
        ]

        all_success = True
        for crate in crates:
            crate_path = zoo_libs / crate
            if crate_path.exists():
                cargo_toml = crate_path / "Cargo.toml"
                if cargo_toml.exists():
                    success = process_cargo_toml(cargo_toml)
                    if not success:
                        all_success = False
                else:
                    print(f"Warning: No Cargo.toml in {crate_path}")
            else:
                print(f"Warning: Crate directory {crate_path} not found")

        sys.exit(0 if all_success else 1)


if __name__ == "__main__":
    main()