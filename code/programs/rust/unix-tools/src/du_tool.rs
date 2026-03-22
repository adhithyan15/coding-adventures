//! # du — Estimate File Space Usage
//!
//! This module implements the business logic for the `du` command.
//! `du` estimates and reports the disk space used by files and
//! directories.
//!
//! ## Default Behavior
//!
//! With no flags, `du` recursively walks directories and prints
//! the disk usage of each directory in 1K-blocks:
//!
//! ```text
//!     $ du
//!     4       ./src
//!     8       ./tests
//!     16      .
//! ```
//!
//! ## Common Flags
//!
//! ```text
//!     Flag    Description
//!     ─────   ──────────────────────────────────────
//!     -a      Show all files, not just directories
//!     -h      Human-readable sizes (1K, 2M, 3G)
//!     -s      Show only a total for each argument
//!     -c      Produce a grand total at the end
//!     -d N    Max depth to report
//! ```
//!
//! ## How Disk Usage Is Calculated
//!
//! On Unix systems, every file occupies disk space in units of
//! blocks (typically 512 bytes or 4096 bytes). The `st_blocks`
//! field from `stat(2)` tells us how many 512-byte blocks a file
//! uses, which may differ from the file's logical size due to:
//!
//! - Block alignment (a 1-byte file still uses one block)
//! - Sparse files (logical size > actual blocks)
//! - File system overhead

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Options controlling the du traversal and output.
#[derive(Debug, Clone)]
pub struct DuOptions {
    /// Show all files, not just directories (-a).
    pub all: bool,
    /// Human-readable output (-h).
    pub human_readable: bool,
    /// SI units — powers of 1000 (--si).
    pub si: bool,
    /// Show only a total for each argument (-s).
    pub summarize: bool,
    /// Produce a grand total (-c).
    pub total: bool,
    /// Maximum depth to report (-d N). None means unlimited.
    pub max_depth: Option<usize>,
    /// Follow symbolic links (-L).
    pub dereference: bool,
}

impl Default for DuOptions {
    fn default() -> Self {
        Self {
            all: false,
            human_readable: false,
            si: false,
            summarize: false,
            total: false,
            max_depth: None,
            dereference: false,
        }
    }
}

/// A single entry in the du output.
#[derive(Debug, Clone)]
pub struct DuEntry {
    /// Disk usage in bytes.
    pub size_bytes: u64,
    /// The path this entry represents.
    pub path: String,
}

/// Calculate disk usage for a path.
///
/// Recursively walks directories, summing up the disk space used
/// by each file. The result is a list of entries, each containing
/// a size and a path.
///
/// # Algorithm
///
/// ```text
///     disk_usage("src/")
///       ├── stat("src/main.rs")  → 4096 bytes (1 block)
///       ├── stat("src/lib.rs")   → 8192 bytes (2 blocks)
///       └── total for "src/"     → 12288 bytes
/// ```
///
/// For directories, we recursively process all children first,
/// then report the directory's cumulative size.
pub fn disk_usage(path: &str, opts: &DuOptions) -> Result<Vec<DuEntry>, String> {
    let mut entries = Vec::new();
    let total = walk_path(path, opts, &mut entries, 0)?;

    // --- Summarize mode: only show the top-level total ---
    if opts.summarize {
        return Ok(vec![DuEntry {
            size_bytes: total,
            path: path.to_string(),
        }]);
    }

    Ok(entries)
}

/// Format a DuEntry for display.
///
/// Converts the size to the appropriate format (blocks, human-readable,
/// or SI) and pairs it with the path.
pub fn format_du_entry(entry: &DuEntry, opts: &DuOptions) -> String {
    let size_str = if opts.human_readable {
        format_human_size(entry.size_bytes)
    } else if opts.si {
        format_si_size(entry.size_bytes)
    } else {
        // Default: 1K-blocks
        format!("{}", bytes_to_1k_blocks(entry.size_bytes))
    };

    format!("{}\t{}", size_str, entry.path)
}

// ---------------------------------------------------------------------------
// Internal traversal
// ---------------------------------------------------------------------------

/// Recursively walk a path, collecting DuEntry records.
///
/// Returns the total size in bytes for the path (including all
/// descendants).
fn walk_path(
    path: &str,
    opts: &DuOptions,
    entries: &mut Vec<DuEntry>,
    depth: usize,
) -> Result<u64, String> {
    let metadata = if opts.dereference {
        std::fs::metadata(path)
    } else {
        // Don't follow symlinks by default
        std::fs::symlink_metadata(path)
    };

    let meta = metadata.map_err(|e| format!("du: cannot access '{}': {}", path, e))?;

    if meta.is_file() || meta.file_type().is_symlink() {
        // --- Single file ---
        let size = file_disk_usage(&meta);

        // Only add individual file entries if -a is set
        if opts.all {
            let should_report = match opts.max_depth {
                Some(max) => depth <= max,
                None => true,
            };
            if should_report {
                entries.push(DuEntry {
                    size_bytes: size,
                    path: path.to_string(),
                });
            }
        }

        return Ok(size);
    }

    if meta.is_dir() {
        // --- Directory: recurse into children ---
        let mut total = 0u64;

        // Add the directory's own overhead
        total += file_disk_usage(&meta);

        let read_dir = std::fs::read_dir(path)
            .map_err(|e| format!("du: cannot read directory '{}': {}", path, e))?;

        for entry_result in read_dir {
            let entry = entry_result
                .map_err(|e| format!("du: error reading entry in '{}': {}", path, e))?;
            let child_path = entry.path();
            let child_str = child_path.to_string_lossy().to_string();

            match walk_path(&child_str, opts, entries, depth + 1) {
                Ok(child_size) => total += child_size,
                Err(e) => {
                    // Print warning but continue (like GNU du)
                    eprintln!("{}", e);
                }
            }
        }

        // --- Report this directory ---
        let should_report = match opts.max_depth {
            Some(max) => depth <= max,
            None => true,
        };

        if should_report {
            entries.push(DuEntry {
                size_bytes: total,
                path: path.to_string(),
            });
        }

        return Ok(total);
    }

    // Other file types (sockets, pipes, etc.) — just get their size
    Ok(file_disk_usage(&meta))
}

/// Get the disk usage of a file from its metadata.
///
/// On Unix, we use `st_blocks` which reports in 512-byte blocks.
/// This gives the actual disk usage, which may differ from the
/// logical file size.
///
/// ```text
///     A 1-byte file might use 4096 bytes on disk (one block).
///     A 1GB sparse file might use 0 bytes on disk.
/// ```
fn file_disk_usage(meta: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // st_blocks is in 512-byte units
        meta.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

/// Format bytes as human-readable (powers of 1024).
fn format_human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "K", "M", "G", "T", "P"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_idx])
    }
}

/// Format bytes as SI units (powers of 1000).
fn format_si_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "k", "M", "G", "T", "P"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1000.0 && unit_idx < UNITS.len() - 1 {
        size /= 1000.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_idx])
    }
}

/// Convert bytes to 1K-blocks.
fn bytes_to_1k_blocks(bytes: u64) -> u64 {
    (bytes + 1023) / 1024
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary directory with some test files.
    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("failed to create temp dir");
        fs::write(dir.path().join("file1.txt"), "hello world\n").unwrap();
        fs::write(dir.path().join("file2.txt"), "another file\n").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/nested.txt"), "nested content\n").unwrap();
        dir
    }

    #[test]
    fn du_directory() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let result = disk_usage(&path, &DuOptions::default());
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty(), "should have at least one entry");
    }

    #[test]
    fn du_summarize() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { summarize: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1, "summarize should produce one entry");
        assert_eq!(entries[0].path, path);
    }

    #[test]
    fn du_all_shows_files() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { all: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        // With -a, we should see files too
        let file_entries: Vec<_> = entries.iter()
            .filter(|e| e.path.ends_with(".txt"))
            .collect();
        assert!(!file_entries.is_empty(), "with -a, should see file entries");
    }

    #[test]
    fn du_max_depth() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { max_depth: Some(0), ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1, "max_depth=0 should show only the top dir");
    }

    #[test]
    fn du_nonexistent_path() {
        let result = disk_usage("/nonexistent/path/12345", &DuOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn du_single_file() {
        let dir = setup_test_dir();
        let file_path = dir.path().join("file1.txt");
        let path = file_path.to_string_lossy().to_string();
        let opts = DuOptions { all: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].size_bytes > 0);
    }

    #[test]
    fn format_du_entry_default() {
        let entry = DuEntry { size_bytes: 4096, path: "test".into() };
        let output = format_du_entry(&entry, &DuOptions::default());
        assert!(output.contains("test"));
        assert!(output.contains("4")); // 4096 bytes = 4 1K-blocks
    }

    #[test]
    fn format_du_entry_human() {
        let entry = DuEntry { size_bytes: 1048576, path: "test".into() };
        let opts = DuOptions { human_readable: true, ..Default::default() };
        let output = format_du_entry(&entry, &opts);
        assert!(output.contains("1.0M"));
    }

    #[test]
    fn format_human_size_values() {
        assert_eq!(format_human_size(500), "500");
        assert_eq!(format_human_size(1024), "1.0K");
        assert_eq!(format_human_size(1048576), "1.0M");
    }

    #[test]
    fn bytes_to_1k_blocks_values() {
        assert_eq!(bytes_to_1k_blocks(0), 0);
        assert_eq!(bytes_to_1k_blocks(1024), 1);
        assert_eq!(bytes_to_1k_blocks(1025), 2);
    }
}
