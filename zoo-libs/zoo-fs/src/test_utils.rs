// Duplicated code. Find a way to share it.

#[cfg(test)]

/// Create a temporary directory and set the NODE_STORAGE_PATH environment variable
/// Return the TempDir object (required so it doesn't get deleted when the function returns)
pub fn testing_create_tempdir_and_set_env_var() -> tempfile::TempDir {
    use zoo_message_primitives::zoo_utils::zoo_path::ZooPath;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    env::set_var("NODE_STORAGE_PATH", dir.path().to_string_lossy().to_string());

    env::set_var(
        "ZOO_TOOLS_RUNNER_DENO_BINARY_PATH",
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/zoo-tools-runner-resources/deno")
            .to_string_lossy()
            .to_string(),
    );

    env::set_var(
        "ZOO_TOOLS_RUNNER_UV_BINARY_PATH",
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/zoo-tools-runner-resources/uv")
            .to_string_lossy()
            .to_string(),
    );

    let zoo_path = ZooPath::from_base_path();

    // Check if the directory exists, and create it if it doesn't
    if !zoo_path.as_path().exists() {
        let _ = fs::create_dir_all(&zoo_path.as_path()).map_err(|e| {
            eprintln!("Failed to create directory {}: {}", zoo_path.as_path().display(), e);
            panic!("Failed to create directory {}: {}", zoo_path.as_path().display(), e);
        });
    }

    dir // Return the TempDir object
}
