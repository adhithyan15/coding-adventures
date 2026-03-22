//! # tee — Read from Stdin, Write to Stdout and Files
//!
//! This module implements the business logic for the `tee` command.
//! The `tee` utility reads from standard input and writes to both
//! standard output and one or more files simultaneously.
//!
//! ## How It Works
//!
//! Think of `tee` like a plumbing T-junction: data flows in one
//! direction but is split to multiple destinations.
//!
//! ```text
//!     stdin ──────┬──────► stdout
//!                 │
//!                 ├──────► file1
//!                 │
//!                 └──────► file2
//! ```
//!
//! ## Append vs Overwrite
//!
//! By default, `tee` overwrites (truncates) the output files. The
//! `-a` flag switches to append mode, preserving existing content.
//!
//! ```text
//!     Mode        Effect
//!     ─────────   ────────────────────────────────────
//!     Overwrite   File is truncated before writing
//!     Append      New data is added after existing content
//! ```

use std::fs::{File, OpenOptions};
use std::io::Write;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write content to multiple files, either appending or overwriting.
///
/// This is the core I/O function of tee. It takes the content read
/// from stdin and writes it to each specified file. The caller is
/// responsible for also writing to stdout.
///
/// # Parameters
///
/// - `content`: The text to write
/// - `files`: Paths to output files
/// - `append`: If true, append to existing files; if false, overwrite
///
/// # Returns
///
/// `Ok(())` on success, or `Err(message)` if any file operation fails.
/// Note that like GNU tee, we attempt to write to all files even if
/// some fail — errors are collected.
///
/// # Example
///
/// ```text
///     tee_content("hello\n", &["out1.txt".into(), "out2.txt".into()], false)
///     // Creates/overwrites out1.txt and out2.txt with "hello\n"
///
///     tee_content("more\n", &["out1.txt".into()], true)
///     // Appends "more\n" to out1.txt
/// ```
pub fn tee_content(content: &str, files: &[String], append: bool) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    for path in files {
        // --- Open the file in the appropriate mode ---
        let file_result = if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
        } else {
            File::create(path)
        };

        match file_result {
            Ok(mut file) => {
                // --- Write content to the file ---
                if let Err(e) = file.write_all(content.as_bytes()) {
                    errors.push(format!("tee: {}: {}", path, e));
                }
            }
            Err(e) => {
                errors.push(format!("tee: {}: {}", path, e));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper to create a temporary file path for testing.
    fn temp_path(name: &str) -> String {
        let dir = std::env::temp_dir();
        dir.join(format!("tee_test_{}", name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn write_to_single_file() {
        let path = temp_path("single");
        tee_content("hello\n", &[path.clone()], false).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn write_to_multiple_files() {
        let p1 = temp_path("multi1");
        let p2 = temp_path("multi2");
        tee_content("data\n", &[p1.clone(), p2.clone()], false).unwrap();
        assert_eq!(fs::read_to_string(&p1).unwrap(), "data\n");
        assert_eq!(fs::read_to_string(&p2).unwrap(), "data\n");
        fs::remove_file(&p1).ok();
        fs::remove_file(&p2).ok();
    }

    #[test]
    fn append_mode() {
        let path = temp_path("append");
        tee_content("first\n", &[path.clone()], false).unwrap();
        tee_content("second\n", &[path.clone()], true).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first\nsecond\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn overwrite_mode() {
        let path = temp_path("overwrite");
        tee_content("original\n", &[path.clone()], false).unwrap();
        tee_content("replaced\n", &[path.clone()], false).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "replaced\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn empty_file_list() {
        // Writing to no files should succeed
        assert!(tee_content("data\n", &[], false).is_ok());
    }

    #[test]
    fn invalid_path_returns_error() {
        let result = tee_content("data\n", &["/nonexistent/dir/file.txt".into()], false);
        assert!(result.is_err());
    }

    #[test]
    fn empty_content() {
        let path = temp_path("empty");
        tee_content("", &[path.clone()], false).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "");
        fs::remove_file(&path).ok();
    }
}
