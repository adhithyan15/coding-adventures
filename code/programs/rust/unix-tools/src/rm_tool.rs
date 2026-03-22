//! # rm — Remove Files or Directories
//!
//! This module implements the business logic for the `rm` command.
//! rm removes files and directories. There is no "undo" — use with care.
//!
//! ## Modes
//!
//! ```text
//!     rm file.txt         Remove a file
//!     rm -r dir/          Recursively remove directory and contents
//!     rm -f missing.txt   Force: ignore nonexistent files
//!     rm -d empty_dir/    Remove empty directory (like rmdir)
//! ```

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Remove a single file or directory.
///
/// # Parameters
/// - `path`: the path to remove
/// - `recursive`: if true, remove directories recursively
/// - `dir_mode`: if true, allow removing empty directories
/// - `force`: if true, silently ignore nonexistent paths
///
/// # Returns
/// `Ok(())` on success, `Err(message)` on failure.
pub fn remove_path(path: &str, recursive: bool, dir_mode: bool, force: bool) -> Result<(), String> {
    let file_path = Path::new(path);

    // Check if the path exists.
    let metadata = match fs::symlink_metadata(file_path) {
        Ok(m) => m,
        Err(_) => {
            if force {
                return Ok(()); // Silently ignore nonexistent files
            }
            return Err(format!("rm: cannot remove '{}': No such file or directory", path));
        }
    };

    if metadata.is_dir() {
        if recursive {
            // Remove the entire directory tree.
            fs::remove_dir_all(file_path)
                .map_err(|e| format!("rm: cannot remove '{}': {}", path, e))
        } else if dir_mode {
            // Remove only if empty (like rmdir).
            fs::remove_dir(file_path)
                .map_err(|e| format!("rm: cannot remove '{}': {}", path, e))
        } else {
            Err(format!("rm: cannot remove '{}': Is a directory", path))
        }
    } else {
        // Regular file or symlink.
        fs::remove_file(file_path)
            .map_err(|e| format!("rm: cannot remove '{}': {}", path, e))
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
        dir.join(format!("rm_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn remove_file() {
        let path = temp_path("file");
        fs::write(&path, "data").unwrap();
        assert!(remove_path(&path, false, false, false).is_ok());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn remove_nonexistent_without_force() {
        let result = remove_path("/tmp/rm_test_nonexistent_xyz", false, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn remove_nonexistent_with_force() {
        let result = remove_path("/tmp/rm_test_nonexistent_xyz", false, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn remove_dir_without_recursive() {
        let path = temp_path("dir_no_r");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::write(format!("{}/file.txt", path), "data").unwrap();
        let result = remove_path(&path, false, false, false);
        assert!(result.is_err());
        fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn remove_dir_recursive() {
        let path = temp_path("dir_r");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(format!("{}/sub", path)).unwrap();
        fs::write(format!("{}/sub/file.txt", path), "data").unwrap();
        assert!(remove_path(&path, true, false, false).is_ok());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn remove_empty_dir_with_d() {
        let path = temp_path("empty_d");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        assert!(remove_path(&path, false, true, false).is_ok());
        assert!(!Path::new(&path).exists());
    }
}
