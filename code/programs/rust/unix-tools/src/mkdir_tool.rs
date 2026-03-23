//! # mkdir — Make Directories
//!
//! This module implements the business logic for the `mkdir` command.
//! The `mkdir` utility creates new directories.
//!
//! ## How It Works
//!
//! ```text
//!     mkdir new_dir           Create a single directory
//!     mkdir -p a/b/c          Create all parent directories as needed
//!     mkdir -v dir            Print a message for each created directory
//! ```
//!
//! ## The -p Flag (Parents)
//!
//! Without -p, creating "a/b/c" fails if "a/b" doesn't exist.
//! With -p, all intermediate directories are created automatically,
//! and it's NOT an error if the directory already exists.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a directory, optionally creating parent directories.
///
/// # Parameters
/// - `path`: the directory path to create
/// - `parents`: if true, create parent directories as needed
///
/// # Returns
/// `Ok(())` on success, or `Err(message)` on failure.
pub fn make_directory(path: &str, parents: bool) -> Result<(), String> {
    let dir_path = Path::new(path);

    if parents {
        // create_dir_all creates the full path including parents.
        // It does NOT error if the directory already exists.
        fs::create_dir_all(dir_path)
            .map_err(|e| format!("mkdir: cannot create directory '{}': {}", path, e))
    } else {
        // create_dir creates only the leaf directory.
        // It errors if the parent doesn't exist or directory already exists.
        fs::create_dir(dir_path)
            .map_err(|e| format!("mkdir: cannot create directory '{}': {}", path, e))
    }
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
        dir.join(format!("mkdir_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn create_single_directory() {
        let path = temp_path("single");
        let _ = fs::remove_dir_all(&path);
        assert!(make_directory(&path, false).is_ok());
        assert!(Path::new(&path).is_dir());
        fs::remove_dir(&path).ok();
    }

    #[test]
    fn create_with_parents() {
        let path = temp_path("parent/child/grandchild");
        let root = temp_path("parent");
        let _ = fs::remove_dir_all(&root);
        assert!(make_directory(&path, true).is_ok());
        assert!(Path::new(&path).is_dir());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn error_without_parents() {
        let path = temp_path("no_parent/child");
        let root = temp_path("no_parent");
        let _ = fs::remove_dir_all(&root);
        let result = make_directory(&path, false);
        assert!(result.is_err());
    }

    #[test]
    fn existing_with_parents_ok() {
        let path = temp_path("existing_p");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        assert!(make_directory(&path, true).is_ok());
        fs::remove_dir(&path).ok();
    }

    #[test]
    fn existing_without_parents_error() {
        let path = temp_path("existing_no_p");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let result = make_directory(&path, false);
        assert!(result.is_err());
        fs::remove_dir(&path).ok();
    }
}
