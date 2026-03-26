//! # mv — Move (Rename) Files
//!
//! This module implements the business logic for the `mv` command.
//! `mv` moves files and directories from one location to another,
//! or renames them in place.
//!
//! ## How It Works
//!
//! Moving a file is fundamentally different from copying:
//!
//! ```text
//!     Rename (same filesystem):
//!         Just update the directory entry — O(1), instant.
//!         The file data stays exactly where it is on disk.
//!
//!     Move (across filesystems):
//!         1. Copy all bytes to the new location
//!         2. Delete the original
//!         This is O(n) where n = file size.
//! ```
//!
//! ## Examples
//!
//! ```text
//!     mv old.txt new.txt          Rename a file
//!     mv file.txt dir/            Move into a directory
//!     mv -n file.txt existing     Don't overwrite existing
//!     mv -f file.txt existing     Force overwrite
//! ```
//!
//! ## Strategy
//!
//! We always try `std::fs::rename` first. If it fails (typically
//! because src and dst are on different filesystems), we fall back
//! to copy-then-delete.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `move_file` behaves.
///
/// ```text
///     Flag             Field        Effect
///     ──────────────   ──────────   ──────────────────────────────
///     -n, --no-clobber no_clobber   Don't overwrite existing files
///     -f, --force      force        Don't prompt before overwriting
///     -v, --verbose    verbose      Print each file as it's moved
/// ```
#[derive(Debug, Clone, Default)]
pub struct MvOptions {
    /// If true, do not overwrite existing destination files.
    pub no_clobber: bool,
    /// If true, do not prompt before overwriting.
    pub force: bool,
    /// If true, print the name of each file as it is moved.
    pub verbose: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Move (rename) a file or directory from `src` to `dst`.
///
/// # Behavior
///
/// ```text
///     Source       Destination        Result
///     ──────────   ─────────────────  ────────────────────────────────
///     file         new path           Renames the file
///     file         existing file      Overwrites (unless no_clobber)
///     file         existing dir       Moves into dir/basename(src)
///     directory    new path           Renames the directory
///     directory    existing dir       Moves into dst/basename(src)
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - The source doesn't exist
/// - A filesystem error occurs during the move
pub fn move_file(src: &str, dst: &str, opts: &MvOptions) -> Result<(), String> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    // --- Verify source exists ---
    if !src_path.exists() {
        return Err(format!(
            "mv: cannot stat '{}': No such file or directory",
            src
        ));
    }

    // --- Determine effective destination ---
    // If destination is an existing directory, move INTO it.
    let effective_dst = if dst_path.is_dir() {
        let filename = src_path
            .file_name()
            .ok_or_else(|| format!("mv: cannot determine filename from '{}'", src))?;
        dst_path.join(filename)
    } else {
        dst_path.to_path_buf()
    };

    // --- Check no_clobber ---
    if opts.no_clobber && effective_dst.exists() {
        // Silently skip — matches GNU mv behavior with -n
        return Ok(());
    }

    // --- Try rename first (fast path) ---
    // std::fs::rename is atomic on the same filesystem.
    // It fails across filesystem boundaries (ErrorKind::CrossesDevices
    // or similar platform-specific errors).
    match fs::rename(src_path, &effective_dst) {
        Ok(()) => Ok(()),
        Err(_rename_err) => {
            // --- Fallback: copy + delete ---
            // This handles cross-filesystem moves. We copy the
            // source to the destination, then remove the source.
            cross_device_move(src_path, &effective_dst, opts)
        }
    }
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/// Perform a cross-device move by copying then deleting.
///
/// ```text
///     Cross-device move:
///         1. Copy src → dst  (preserving directory structure)
///         2. Delete src      (only after successful copy)
/// ```
///
/// If the copy succeeds but the delete fails, we return an error
/// but the data is safely at the destination.
fn cross_device_move(src: &Path, dst: &Path, _opts: &MvOptions) -> Result<(), String> {
    if src.is_dir() {
        // --- Move directory: recursive copy + delete ---
        copy_dir_recursive(src, dst)?;
        fs::remove_dir_all(src)
            .map_err(|e| format!("mv: cannot remove '{}': {}", src.display(), e))?;
    } else {
        // --- Move file: copy + delete ---
        fs::copy(src, dst)
            .map_err(|e| format!("mv: cannot copy '{}' to '{}': {}", src.display(), dst.display(), e))?;
        fs::remove_file(src)
            .map_err(|e| format!("mv: cannot remove '{}': {}", src.display(), e))?;
    }
    Ok(())
}

/// Recursively copy a directory tree (used for cross-device moves).
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("mv: cannot create directory '{}': {}", dst.display(), e))?;
    }

    let entries = fs::read_dir(src)
        .map_err(|e| format!("mv: cannot read directory '{}': {}", src.display(), e))?;

    for entry_result in entries {
        let entry = entry_result
            .map_err(|e| format!("mv: error reading entry in '{}': {}", src.display(), e))?;
        let entry_path = entry.path();
        let dest_entry = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_entry)?;
        } else {
            fs::copy(&entry_path, &dest_entry).map_err(|e| {
                format!(
                    "mv: cannot copy '{}' to '{}': {}",
                    entry_path.display(),
                    dest_entry.display(),
                    e
                )
            })?;
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
            .join(format!("mv_test_{}", name))
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
    fn move_rename_file() {
        let src = temp_path("rename_src");
        let dst = temp_path("rename_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "data").unwrap();
        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "data");

        cleanup(&dst);
    }

    #[test]
    fn move_nonexistent_source_fails() {
        let result = move_file(
            "/tmp/mv_test_nonexistent_xyz",
            "/tmp/mv_test_dst",
            &MvOptions::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file"));
    }

    #[test]
    fn no_clobber_prevents_overwrite() {
        let src = temp_path("noclobber_src");
        let dst = temp_path("noclobber_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "original data").unwrap();

        let opts = MvOptions { no_clobber: true, ..Default::default() };
        assert!(move_file(&src, &dst, &opts).is_ok());
        // Destination should keep original content
        assert_eq!(fs::read_to_string(&dst).unwrap(), "original data");
        // Source should still exist (move was skipped)
        assert!(Path::new(&src).exists());

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn move_into_directory() {
        let src = temp_path("into_dir_src");
        let dst_dir = temp_path("into_dir_dst");
        cleanup(&src);
        cleanup(&dst_dir);

        fs::write(&src, "data").unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        assert!(move_file(&src, &dst_dir, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());

        let expected = Path::new(&dst_dir).join("mv_test_into_dir_src");
        assert!(expected.exists());
        assert_eq!(fs::read_to_string(expected).unwrap(), "data");

        cleanup(&dst_dir);
    }

    #[test]
    fn move_overwrites_by_default() {
        let src = temp_path("overwrite_src");
        let dst = temp_path("overwrite_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "old data").unwrap();

        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new data");

        cleanup(&dst);
    }

    #[test]
    fn move_directory() {
        let src = temp_path("dir_mv_src");
        let dst = temp_path("dir_mv_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::create_dir_all(format!("{}/sub", src)).unwrap();
        fs::write(format!("{}/sub/file.txt", src), "nested").unwrap();

        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(
            fs::read_to_string(format!("{}/sub/file.txt", dst)).unwrap(),
            "nested"
        );

        cleanup(&dst);
    }
}
