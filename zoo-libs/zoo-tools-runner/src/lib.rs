//! Zoo Tools Runner - Thin wrapper around hanzo_tools_runner
//!
//! This crate re-exports hanzo_tools_runner with zoo-specific extensions.
//! Follows DRY principle - single source of truth from hanzo implementation.

// Re-export everything from hanzo_tools_runner at the root level
pub use hanzo_tools_runner::*;

// Re-export tools module with zoo extensions
pub mod tools {
    // Re-export all hanzo tools
    pub use hanzo_tools_runner::tools::*;

    // Add zoo-specific zoo_node_location module
    pub mod zoo_node_location {
        use serde::{Deserialize, Serialize};

        /// Zoo-specific node location configuration
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        pub struct ZooNodeLocation {
            pub protocol: String,
            pub host: String,
            pub port: u16,
        }

        impl Default for ZooNodeLocation {
            fn default() -> Self {
                Self {
                    protocol: "http".to_string(),
                    host: "127.0.0.1".to_string(),
                    port: 9550,
                }
            }
        }
    }
}
