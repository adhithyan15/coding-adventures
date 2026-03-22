//! # realpath — Print the Resolved Absolute File Name
//!
//! This module implements the business logic for the `realpath` command.
//! The `realpath` utility resolves symbolic links, `.` and `..` references,
//! and extra slashes to produce the canonical absolute path.
//!
//! ## Modes
//!
//! ```text
//!     Default:    last component need not exist
//!     -e:         ALL components must exist (strict)
//!     -m:         NO components need exist (lenient)
//!     -s:         Don't follow symlinks
//! ```

use std::env;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Resolve a path to its canonical absolute form.
///
/// # Parameters
/// - `path`: the path to resolve
/// - `must_exist`: if true, all components must exist (-e mode)
/// - `no_exist_ok`: if true, no components need exist (-m mode)
/// - `no_symlinks`: if true, don't follow symlinks (-s mode)
pub fn resolve_path(
    path: &str,
    must_exist: bool,
    no_exist_ok: bool,
    no_symlinks: bool,
) -> Result<String, String> {
    let input = Path::new(path);

    if must_exist {
        // -e mode: all components must exist. Use canonicalize which
        // requires the path to exist.
        let canonical = input
            .canonicalize()
            .map_err(|_| format!("{}: No such file or directory", path))?;
        return Ok(canonical.to_string_lossy().into_owned());
    }

    if no_exist_ok {
        // -m mode: compute the absolute path without checking existence.
        let abs = make_absolute(input)?;
        return Ok(clean_path(&abs));
    }

    if no_symlinks {
        // -s mode: make absolute and clean, but don't follow symlinks.
        let abs = make_absolute(input)?;
        return Ok(clean_path(&abs));
    }

    // Default mode: try to canonicalize. If it fails (last component
    // doesn't exist), fall back to absolute + clean.
    match input.canonicalize() {
        Ok(canonical) => Ok(canonical.to_string_lossy().into_owned()),
        Err(_) => {
            let abs = make_absolute(input)?;
            Ok(clean_path(&abs))
        }
    }
}

/// Make a path absolute by prepending the current directory if needed.
fn make_absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = env::current_dir()
            .map_err(|e| format!("realpath: cannot determine cwd: {}", e))?;
        Ok(cwd.join(path))
    }
}

/// Clean a path by resolving `.` and `..` components without touching
/// the filesystem. This is a logical (not physical) path cleanup.
fn clean_path(path: &PathBuf) -> String {
    let mut components: Vec<String> = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {} // skip "."
            std::path::Component::ParentDir => {
                // Go up one level unless we're at root.
                if !components.is_empty()
                    && components.last() != Some(&"/".to_string())
                    && components.last() != Some(&"..".to_string())
                {
                    components.pop();
                }
            }
            _ => {
                components.push(component.as_os_str().to_string_lossy().into_owned());
            }
        }
    }

    if components.is_empty() {
        "/".to_string()
    } else if components[0] == "/" {
        format!("/{}", components[1..].join("/"))
    } else {
        components.join("/")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_current_dir() {
        let result = resolve_path(".", false, false, false);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(Path::new(&path).is_absolute());
    }

    #[test]
    fn resolve_missing_with_m() {
        let result = resolve_path("/nonexistent/path/to/file", false, true, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/nonexistent/path/to/file");
    }

    #[test]
    fn resolve_missing_with_e_fails() {
        let result = resolve_path("/nonexistent/path", true, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_root() {
        let result = resolve_path("/", false, false, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/");
    }

    #[test]
    fn clean_path_with_dots() {
        let path = PathBuf::from("/a/b/../c/./d");
        let result = clean_path(&path);
        assert_eq!(result, "/a/c/d");
    }
}
