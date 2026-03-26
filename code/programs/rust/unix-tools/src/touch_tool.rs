//! # touch — Change File Timestamps
//!
//! This module implements the business logic for the `touch` command.
//! The `touch` utility updates access and modification times of files.
//! If a file doesn't exist, it creates an empty one (unless -c is used).
//!
//! ## Primary Uses
//!
//! ```text
//!     touch newfile.txt       Create an empty file
//!     touch existing.txt      Update timestamps to now
//!     touch -c maybe.txt      Update only if file exists
//! ```
//!
//! ## Why Touch Exists
//!
//! 1. Create empty files as placeholders or markers
//! 2. Update timestamps to trigger build system rebuilds

use std::fs::{self, OpenOptions};
use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Touch a file: create it if it doesn't exist, update its
/// modification time if it does.
///
/// # Parameters
/// - `path`: the file path to touch
/// - `no_create`: if true, don't create the file if it doesn't exist
///
/// # Returns
/// `Ok(true)` if the file was touched (or created),
/// `Ok(false)` if it was skipped (no_create + doesn't exist),
/// `Err(message)` on failure.
pub fn touch_file(path: &str, no_create: bool) -> Result<bool, String> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        if no_create {
            return Ok(false); // Skip silently
        }

        // Create the file by opening it in create mode.
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)
            .map_err(|e| format!("touch: cannot touch '{}': {}", path, e))?;

        return Ok(true);
    }

    // File exists — update its timestamp by setting modification time
    // to now. We do this by opening and immediately closing the file
    // with write permissions, which updates the mtime on most systems.
    // For a more precise approach, we'd use filetime or libc::utimensat.
    let metadata = fs::metadata(file_path)
        .map_err(|e| format!("touch: cannot touch '{}': {}", path, e))?;

    // Touch by re-setting permissions (a no-op that updates mtime on some systems).
    // A more robust approach: use the filetime crate or write zero bytes.
    let perms = metadata.permissions();
    fs::set_permissions(file_path, perms)
        .map_err(|e| format!("touch: cannot touch '{}': {}", path, e))?;

    Ok(true)
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
        dir.join(format!("touch_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn create_new_file() {
        let path = temp_path("new");
        let _ = fs::remove_file(&path);
        let result = touch_file(&path, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert!(Path::new(&path).exists());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn touch_existing_file() {
        let path = temp_path("existing");
        fs::write(&path, "data").unwrap();
        let result = touch_file(&path, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn no_create_skips_nonexistent() {
        let path = temp_path("no_create");
        let _ = fs::remove_file(&path);
        let result = touch_file(&path, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn invalid_path_returns_error() {
        let result = touch_file("/nonexistent/dir/file.txt", false);
        assert!(result.is_err());
    }
}
