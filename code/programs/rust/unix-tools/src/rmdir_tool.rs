//! # rmdir — Remove Empty Directories
//!
//! This module implements the business logic for the `rmdir` command.
//! Unlike `rm -r`, rmdir only removes empty directories — a safety feature.
//!
//! ## How It Works
//!
//! ```text
//!     rmdir empty_dir         Remove a single empty directory
//!     rmdir -p a/b/c          Remove c, then b, then a
//! ```
//!
//! ## The -p Flag (Parents)
//!
//! With -p, rmdir removes the directory AND its ancestors, working
//! from deepest to shallowest. Each ancestor must also be empty.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Remove an empty directory.
///
/// Returns `Ok(())` if the directory was successfully removed,
/// or `Err(message)` if it couldn't be removed (e.g., not empty).
pub fn remove_directory(path: &str) -> Result<(), String> {
    fs::remove_dir(path)
        .map_err(|e| format!("rmdir: failed to remove '{}': {}", path, e))
}

/// Remove a directory and its parent directories (like `rmdir -p`).
///
/// Removes the given path, then removes its parent, grandparent, etc.
/// Stops when a removal fails or when reaching the root.
pub fn remove_with_parents(path: &str) -> Result<(), String> {
    // First remove the deepest directory.
    remove_directory(path)?;

    // Then walk up the parent chain.
    let mut current = Path::new(path);
    loop {
        match current.parent() {
            Some(parent) if parent != Path::new("") && parent != Path::new("/") && parent != Path::new(".") => {
                let parent_str = parent.to_string_lossy();
                if let Err(_) = fs::remove_dir(parent.to_path_buf()) {
                    // Stop on first failure (parent might not be empty).
                    break;
                }
                current = parent;
                let _ = parent_str; // use the binding
            }
            _ => break,
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> String {
        let dir = env::temp_dir();
        dir.join(format!("rmdir_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn remove_empty() {
        let path = temp_path("empty");
        fs::create_dir_all(&path).unwrap();
        assert!(remove_directory(&path).is_ok());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn remove_non_empty_fails() {
        let path = temp_path("nonempty");
        fs::create_dir_all(&path).unwrap();
        let file = Path::new(&path).join("file.txt");
        fs::write(&file, "data").unwrap();
        let result = remove_directory(&path);
        assert!(result.is_err());
        fs::remove_file(&file).ok();
        fs::remove_dir(&path).ok();
    }

    #[test]
    fn remove_nonexistent_fails() {
        let result = remove_directory("/tmp/rmdir_test_nonexistent_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn remove_with_parents_works() {
        let root = temp_path("parent_rm");
        let nested = format!("{}/a/b/c", root);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&nested).unwrap();
        assert!(remove_with_parents(&nested).is_ok());
        // At least the nested path should be gone
        assert!(!Path::new(&nested).exists());
    }
}
