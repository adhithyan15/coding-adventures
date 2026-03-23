//! # ln — Make Links Between Files
//!
//! This module implements the business logic for the `ln` command.
//!
//! ## Two Types of Links
//!
//! ```text
//!     Hard link (default):
//!         "orig" ──┐
//!                  ├──► inode 42 (same data)
//!         "link" ──┘
//!
//!     Symbolic link (-s):
//!         "link" ──► symlink file ──► "target"
//! ```
//!
//! Hard links share the same inode — both names are equally "real."
//! Symbolic links are special files containing a path to another file.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a link (hard or symbolic).
///
/// # Parameters
/// - `target`: the file to link to
/// - `link_name`: the name of the new link
/// - `symbolic`: if true, create a symbolic link; otherwise hard link
/// - `force`: if true, remove existing destination first
pub fn create_link(target: &str, link_name: &str, symbolic: bool, force: bool) -> Result<(), String> {
    let link_path = Path::new(link_name);

    // Remove existing file if force mode.
    if force && link_path.exists() {
        fs::remove_file(link_path)
            .map_err(|e| format!("ln: cannot remove '{}': {}", link_name, e))?;
    }

    if symbolic {
        // Create a symbolic link. On Unix, this uses symlink(2).
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link_name)
                .map_err(|e| format!("ln: failed to create symbolic link '{}': {}", link_name, e))
        }
        #[cfg(not(unix))]
        {
            Err(format!("ln: symbolic links not supported on this platform"))
        }
    } else {
        // Create a hard link. Both names point to the same inode.
        fs::hard_link(target, link_name)
            .map_err(|e| format!("ln: failed to create hard link '{}': {}", link_name, e))
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
        dir.join(format!("ln_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn create_hard_link() {
        let target = temp_path("hard_target");
        let link = temp_path("hard_link");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
        fs::write(&target, "hello").unwrap();

        let result = create_link(&target, &link, false, false);
        assert!(result.is_ok());

        let content = fs::read_to_string(&link).unwrap();
        assert_eq!(content, "hello");

        fs::remove_file(&target).ok();
        fs::remove_file(&link).ok();
    }

    #[cfg(unix)]
    #[test]
    fn create_symbolic_link() {
        let target = temp_path("sym_target");
        let link = temp_path("sym_link");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
        fs::write(&target, "hello").unwrap();

        let result = create_link(&target, &link, true, false);
        assert!(result.is_ok());

        let link_target = fs::read_link(&link).unwrap();
        assert_eq!(link_target.to_string_lossy(), target);

        fs::remove_file(&target).ok();
        fs::remove_file(&link).ok();
    }

    #[test]
    fn force_overwrites_existing() {
        let target = temp_path("force_target");
        let link = temp_path("force_link");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
        fs::write(&target, "new").unwrap();
        fs::write(&link, "old").unwrap();

        let result = create_link(&target, &link, false, true);
        assert!(result.is_ok());

        let content = fs::read_to_string(&link).unwrap();
        assert_eq!(content, "new");

        fs::remove_file(&target).ok();
        fs::remove_file(&link).ok();
    }

    #[test]
    fn nonexistent_target_hard_link_fails() {
        let link = temp_path("bad_hard");
        let _ = fs::remove_file(&link);
        let result = create_link("/nonexistent/target", &link, false, false);
        assert!(result.is_err());
    }
}
