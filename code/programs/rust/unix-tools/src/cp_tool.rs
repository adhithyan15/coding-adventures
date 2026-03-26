//! # cp — Copy Files and Directories
//!
//! This module implements the business logic for the `cp` command.
//! `cp` copies files and directories from one location to another.
//!
//! ## How It Works
//!
//! At its simplest, copying is a two-step process:
//!
//! ```text
//!     1. Read all bytes from the SOURCE file
//!     2. Write those bytes to the DESTINATION file
//! ```
//!
//! But real-world copying has many edge cases:
//!
//! ```text
//!     cp file.txt backup.txt        Copy a single file
//!     cp -r dir/ backup/            Copy a directory recursively
//!     cp -n file.txt existing.txt   Don't overwrite existing files
//!     cp -f file.txt readonly.txt   Force overwrite even if read-only
//! ```
//!
//! ## Key Design Decisions
//!
//! - We use `std::fs::copy` for single files, which preserves file
//!   contents but not necessarily permissions on all platforms.
//! - Recursive copy uses manual recursion with `std::fs::read_dir`
//!   to walk directory trees.
//! - The `no_clobber` option prevents accidental overwrites — a common
//!   safety measure in production scripts.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `copy_file` behaves.
///
/// Each field corresponds to a command-line flag from the cp spec:
///
/// ```text
///     Flag             Field          Effect
///     ──────────────   ────────────   ──────────────────────────────
///     -R, --recursive  recursive      Copy directories recursively
///     -n, --no-clobber no_clobber     Don't overwrite existing files
///     -f, --force      force          Remove destination before copy
///     -v, --verbose    verbose        Print each file as it's copied
/// ```
#[derive(Debug, Clone, Default)]
pub struct CpOptions {
    /// If true, copy directories and their contents recursively.
    pub recursive: bool,
    /// If true, do not overwrite existing destination files.
    pub no_clobber: bool,
    /// If true, remove the destination file before copying if it
    /// cannot be opened for writing.
    pub force: bool,
    /// If true, print the name of each file as it is copied.
    pub verbose: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Copy a file or directory from `src` to `dst`.
///
/// # Behavior
///
/// ```text
///     Source       Destination        Result
///     ──────────   ─────────────────  ────────────────────────────────
///     file         new path           Creates a copy at dst
///     file         existing file      Overwrites (unless no_clobber)
///     file         existing dir       Copies into dir/basename(src)
///     directory    new path           Copies tree (requires recursive)
///     directory    existing dir       Copies tree into dst/basename(src)
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - The source doesn't exist
/// - Trying to copy a directory without `recursive: true`
/// - A filesystem error occurs during the copy
pub fn copy_file(src: &str, dst: &str, opts: &CpOptions) -> Result<(), String> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    // --- Verify source exists ---
    if !src_path.exists() {
        return Err(format!("cp: cannot stat '{}': No such file or directory", src));
    }

    // --- Determine effective destination ---
    // If the destination is an existing directory, copy INTO it.
    // For example: cp file.txt dir/ => dir/file.txt
    let effective_dst = if dst_path.is_dir() {
        let filename = src_path
            .file_name()
            .ok_or_else(|| format!("cp: cannot determine filename from '{}'", src))?;
        dst_path.join(filename)
    } else {
        dst_path.to_path_buf()
    };

    // --- Handle directories ---
    if src_path.is_dir() {
        if !opts.recursive {
            return Err(format!(
                "cp: -r not specified; omitting directory '{}'",
                src
            ));
        }
        return copy_directory_recursive(src_path, &effective_dst, opts);
    }

    // --- Handle files ---
    copy_single_file(src_path, &effective_dst, opts)
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/// Copy a single file, respecting no_clobber and force options.
///
/// The decision tree:
///
/// ```text
///     Does destination exist?
///     ├── No  → copy
///     └── Yes
///         ├── no_clobber? → skip (return Ok)
///         ├── force?      → remove destination, then copy
///         └── otherwise   → overwrite
/// ```
fn copy_single_file(src: &Path, dst: &Path, opts: &CpOptions) -> Result<(), String> {
    // --- Check no_clobber ---
    if opts.no_clobber && dst.exists() {
        // Silently skip — this matches GNU cp behavior with -n
        return Ok(());
    }

    // --- Force: remove destination if it exists ---
    if opts.force && dst.exists() {
        fs::remove_file(dst)
            .map_err(|e| format!("cp: cannot remove '{}': {}", dst.display(), e))?;
    }

    // --- Ensure parent directory exists ---
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            return Err(format!(
                "cp: cannot create regular file '{}': No such file or directory",
                dst.display()
            ));
        }
    }

    // --- Perform the copy ---
    // std::fs::copy copies file contents and permissions.
    // It returns the number of bytes copied, which we discard.
    fs::copy(src, dst)
        .map_err(|e| format!("cp: cannot copy '{}' to '{}': {}", src.display(), dst.display(), e))?;

    Ok(())
}

/// Recursively copy a directory tree.
///
/// Algorithm:
///
/// ```text
///     copy_directory_recursive(src_dir, dst_dir):
///         1. Create dst_dir if it doesn't exist
///         2. For each entry in src_dir:
///             a. If it's a file → copy_single_file
///             b. If it's a directory → recurse
/// ```
///
/// This is a depth-first traversal of the source tree, creating
/// the mirror structure at the destination.
fn copy_directory_recursive(src: &Path, dst: &Path, opts: &CpOptions) -> Result<(), String> {
    // --- Create destination directory ---
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("cp: cannot create directory '{}': {}", dst.display(), e))?;
    }

    // --- Iterate over source entries ---
    let entries = fs::read_dir(src)
        .map_err(|e| format!("cp: cannot read directory '{}': {}", src.display(), e))?;

    for entry_result in entries {
        let entry = entry_result
            .map_err(|e| format!("cp: error reading entry in '{}': {}", src.display(), e))?;
        let entry_path = entry.path();
        let dest_entry = dst.join(entry.file_name());

        if entry_path.is_dir() {
            // --- Recurse into subdirectory ---
            copy_directory_recursive(&entry_path, &dest_entry, opts)?;
        } else {
            // --- Copy the file ---
            copy_single_file(&entry_path, &dest_entry, opts)?;
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
        env::temp_dir()
            .join(format!("cp_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    fn cleanup(path: &str) {
        let p = Path::new(path);
        if p.is_dir() {
            let _ = fs::remove_dir_all(p);
        } else {
            let _ = fs::remove_file(p);
        }
    }

    #[test]
    fn copy_single_file_basic() {
        let src = temp_path("src_basic");
        let dst = temp_path("dst_basic");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "hello world").unwrap();
        let opts = CpOptions::default();
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello world");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_nonexistent_source_fails() {
        let result = copy_file("/tmp/cp_test_nonexistent_xyz", "/tmp/cp_test_dst", &CpOptions::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file"));
    }

    #[test]
    fn no_clobber_prevents_overwrite() {
        let src = temp_path("src_noclobber");
        let dst = temp_path("dst_noclobber");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new content").unwrap();
        fs::write(&dst, "original content").unwrap();

        let opts = CpOptions { no_clobber: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        // Destination should keep its original content
        assert_eq!(fs::read_to_string(&dst).unwrap(), "original content");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_directory_without_recursive_fails() {
        let src_dir = temp_path("src_dir_norec");
        cleanup(&src_dir);
        fs::create_dir_all(&src_dir).unwrap();

        let result = copy_file(&src_dir, "/tmp/cp_test_dst_dir", &CpOptions::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not specified"));

        cleanup(&src_dir);
    }

    #[test]
    fn copy_directory_recursive() {
        let src_dir = temp_path("src_dir_rec");
        let dst_dir = temp_path("dst_dir_rec");
        cleanup(&src_dir);
        cleanup(&dst_dir);

        fs::create_dir_all(format!("{}/sub", src_dir)).unwrap();
        fs::write(format!("{}/file1.txt", src_dir), "file1").unwrap();
        fs::write(format!("{}/sub/file2.txt", src_dir), "file2").unwrap();

        let opts = CpOptions { recursive: true, ..Default::default() };
        assert!(copy_file(&src_dir, &dst_dir, &opts).is_ok());

        assert!(Path::new(&dst_dir).is_dir());
        assert_eq!(
            fs::read_to_string(format!("{}/file1.txt", dst_dir)).unwrap(),
            "file1"
        );
        assert_eq!(
            fs::read_to_string(format!("{}/sub/file2.txt", dst_dir)).unwrap(),
            "file2"
        );

        cleanup(&src_dir);
        cleanup(&dst_dir);
    }

    #[test]
    fn copy_into_existing_directory() {
        let src = temp_path("src_into_dir");
        let dst_dir = temp_path("dst_into_dir");
        cleanup(&src);
        cleanup(&dst_dir);

        fs::write(&src, "data").unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        let opts = CpOptions::default();
        assert!(copy_file(&src, &dst_dir, &opts).is_ok());

        let expected = Path::new(&dst_dir).join(format!("cp_test_src_into_dir"));
        assert!(expected.exists());

        cleanup(&src);
        cleanup(&dst_dir);
    }

    #[test]
    fn force_removes_destination_before_copy() {
        let src = temp_path("src_force");
        let dst = temp_path("dst_force");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "old data").unwrap();

        let opts = CpOptions { force: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new data");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_empty_directory() {
        let src = temp_path("src_empty_dir");
        let dst = temp_path("dst_empty_dir");
        cleanup(&src);
        cleanup(&dst);

        fs::create_dir_all(&src).unwrap();
        let opts = CpOptions { recursive: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert!(Path::new(&dst).is_dir());

        cleanup(&src);
        cleanup(&dst);
    }
}
