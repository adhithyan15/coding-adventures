//! # ls — List Directory Contents
//!
//! This module implements the business logic for the `ls` command.
//! `ls` lists information about files and directories.
//!
//! ## How It Works
//!
//! ```text
//!     ls              List current directory (non-hidden files)
//!     ls -a           List all files including hidden (. prefixed)
//!     ls -l           Long listing format (permissions, size, date)
//!     ls -R           Recursively list subdirectories
//!     ls -S           Sort by file size
//!     ls -t           Sort by modification time
//! ```
//!
//! ## The FileEntry Struct
//!
//! Rather than printing directly, we return a `Vec<FileEntry>` containing
//! structured information about each file. This separation allows the
//! caller (main.rs) to format the output however it wants — one file
//! per line, long format, columnar, etc.
//!
//! ## Sorting
//!
//! By default, `ls` sorts entries alphabetically by name. Other sort
//! modes override this:
//!
//! ```text
//!     Mode          Flag    Comparison key
//!     ────────────  ─────   ──────────────────
//!     Alphabetical  (none)  file name
//!     By size       -S      file size (largest first)
//!     By time       -t      modification time (newest first)
//!     By extension  -X      file extension alphabetically
//!     Unsorted      -U      directory order (as returned by OS)
//! ```

use std::fs;
use std::path::Path;
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Data Types
// ---------------------------------------------------------------------------

/// Represents a single directory entry with its metadata.
///
/// This is the "view model" for ls output. Each field captures
/// one piece of information that various ls flags might display.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The file name (just the last component, not the full path).
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Whether this entry is a symlink.
    pub is_symlink: bool,
    /// Whether the file is read-only (no write permission).
    pub readonly: bool,
    /// Modification time as seconds since the Unix epoch.
    /// `None` if the time could not be determined.
    pub modified: Option<u64>,
}

/// Options that control how `list_directory` collects and sorts entries.
///
/// ```text
///     Flag              Field           Effect
///     ──────────────    ──────────────  ──────────────────────────────
///     -a, --all         show_all        Include hidden files (. prefix)
///     -A, --almost-all  almost_all      Include hidden except . and ..
///     -R, --recursive   recursive       List subdirectories recursively
///     -S                sort_by_size    Sort by file size
///     -t                sort_by_time    Sort by modification time
///     -X                sort_by_ext     Sort by file extension
///     -U                unsorted        Don't sort, use directory order
///     -r, --reverse     reverse         Reverse sort order
/// ```
#[derive(Debug, Clone, Default)]
pub struct LsOptions {
    /// Show all files, including those starting with '.'
    pub show_all: bool,
    /// Show all files except '.' and '..'
    pub almost_all: bool,
    /// Recurse into subdirectories.
    pub recursive: bool,
    /// Sort by file size (largest first).
    pub sort_by_size: bool,
    /// Sort by modification time (newest first).
    pub sort_by_time: bool,
    /// Sort by file extension alphabetically.
    pub sort_by_ext: bool,
    /// Do not sort; list in directory order.
    pub unsorted: bool,
    /// Reverse the sort order.
    pub reverse: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// List the contents of a directory and return structured entries.
///
/// # Algorithm
///
/// ```text
///     list_directory(path, opts):
///         1. Read all entries from the directory
///         2. Filter: skip hidden files unless -a or -A
///         3. Collect metadata for each entry
///         4. Sort according to options
///         5. Reverse if -r is set
///         6. Return Vec<FileEntry>
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// - The path doesn't exist
/// - The path isn't a directory
/// - A permission error occurs
pub fn list_directory(path: &str, opts: &LsOptions) -> Result<Vec<FileEntry>, String> {
    let dir_path = Path::new(path);

    if !dir_path.exists() {
        return Err(format!(
            "ls: cannot access '{}': No such file or directory",
            path
        ));
    }

    // --- If it's a file, return just that file's info ---
    if !dir_path.is_dir() {
        let entry = file_entry_from_path(dir_path)?;
        return Ok(vec![entry]);
    }

    // --- Read directory entries ---
    let read_dir = fs::read_dir(dir_path)
        .map_err(|e| format!("ls: cannot open directory '{}': {}", path, e))?;

    let mut entries = Vec::new();

    for result in read_dir {
        let dir_entry = result
            .map_err(|e| format!("ls: error reading entry in '{}': {}", path, e))?;

        let name = dir_entry.file_name().to_string_lossy().into_owned();

        // --- Filter hidden files ---
        // Files starting with '.' are "hidden" on Unix.
        // -a shows everything, -A shows everything except . and ..
        if name.starts_with('.') {
            if !opts.show_all && !opts.almost_all {
                continue; // Skip hidden files
            }
            // With -A, we still skip . and .. (but those don't appear
            // in read_dir results anyway, so this is mostly documentation)
        }

        let entry = file_entry_from_dir_entry(&dir_entry, &name)?;
        entries.push(entry);
    }

    // --- Sort ---
    if !opts.unsorted {
        sort_entries(&mut entries, opts);
    }

    // --- Reverse ---
    if opts.reverse {
        entries.reverse();
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/// Create a FileEntry from a Path (for listing a single file).
fn file_entry_from_path(path: &Path) -> Result<FileEntry, String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("ls: cannot stat '{}': {}", path.display(), e))?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    Ok(FileEntry {
        name,
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        is_symlink: path.is_symlink(),
        readonly: metadata.permissions().readonly(),
        modified: modified_secs(&metadata),
    })
}

/// Create a FileEntry from a DirEntry.
fn file_entry_from_dir_entry(
    dir_entry: &fs::DirEntry,
    name: &str,
) -> Result<FileEntry, String> {
    let metadata = dir_entry
        .metadata()
        .map_err(|e| format!("ls: cannot stat '{}': {}", name, e))?;

    Ok(FileEntry {
        name: name.to_string(),
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        is_symlink: dir_entry.path().is_symlink(),
        readonly: metadata.permissions().readonly(),
        modified: modified_secs(&metadata),
    })
}

/// Extract modification time as seconds since Unix epoch.
///
/// Returns `None` if the system doesn't support file timestamps
/// or if the time is before the epoch.
fn modified_secs(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Sort entries according to the given options.
///
/// ```text
///     Priority: -S (size) > -t (time) > -X (extension) > name (default)
/// ```
fn sort_entries(entries: &mut [FileEntry], opts: &LsOptions) {
    if opts.sort_by_size {
        // Sort by size, largest first (descending)
        entries.sort_by(|a, b| b.size.cmp(&a.size));
    } else if opts.sort_by_time {
        // Sort by modification time, newest first (descending)
        entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    } else if opts.sort_by_ext {
        // Sort by extension, then by name within same extension
        entries.sort_by(|a, b| {
            let ext_a = extension_of(&a.name);
            let ext_b = extension_of(&b.name);
            ext_a.cmp(&ext_b).then(a.name.cmp(&b.name))
        });
    } else {
        // Default: sort alphabetically by name (case-sensitive)
        entries.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

/// Extract the file extension for sorting.
///
/// ```text
///     "file.txt"   → "txt"
///     "Makefile"   → ""
///     ".hidden"    → ""  (dotfiles have no extension)
///     "file.tar.gz" → "gz"
/// ```
fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_dir(name: &str) -> String {
        let p = env::temp_dir().join(format!("ls_test_{}", name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn cleanup(path: &str) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn list_empty_directory() {
        let dir = temp_dir("empty");
        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        assert!(result.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn list_nonexistent_directory() {
        let result = list_directory("/tmp/ls_test_nonexistent_xyz", &LsOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn list_files_sorted_by_name() {
        let dir = temp_dir("sorted");
        fs::write(format!("{}/cherry.txt", dir), "").unwrap();
        fs::write(format!("{}/apple.txt", dir), "").unwrap();
        fs::write(format!("{}/banana.txt", dir), "").unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["apple.txt", "banana.txt", "cherry.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn hidden_files_excluded_by_default() {
        let dir = temp_dir("hidden");
        fs::write(format!("{}/.hidden", dir), "").unwrap();
        fs::write(format!("{}/visible", dir), "").unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["visible"]);
        cleanup(&dir);
    }

    #[test]
    fn show_all_includes_hidden() {
        let dir = temp_dir("show_all");
        fs::write(format!("{}/.hidden", dir), "").unwrap();
        fs::write(format!("{}/visible", dir), "").unwrap();

        let opts = LsOptions { show_all: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&".hidden"));
        assert!(names.contains(&"visible"));
        cleanup(&dir);
    }

    #[test]
    fn sort_by_size() {
        let dir = temp_dir("by_size");
        fs::write(format!("{}/small", dir), "a").unwrap();
        fs::write(format!("{}/big", dir), "aaaaaaaaaa").unwrap();
        fs::write(format!("{}/medium", dir), "aaaaa").unwrap();

        let opts = LsOptions { sort_by_size: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["big", "medium", "small"]);
        cleanup(&dir);
    }

    #[test]
    fn reverse_sort() {
        let dir = temp_dir("reverse");
        fs::write(format!("{}/a.txt", dir), "").unwrap();
        fs::write(format!("{}/b.txt", dir), "").unwrap();
        fs::write(format!("{}/c.txt", dir), "").unwrap();

        let opts = LsOptions { reverse: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["c.txt", "b.txt", "a.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn file_entry_captures_metadata() {
        let dir = temp_dir("metadata");
        fs::write(format!("{}/file.txt", dir), "hello").unwrap();
        fs::create_dir(format!("{}/subdir", dir)).unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let file = result.iter().find(|e| e.name == "file.txt").unwrap();
        assert_eq!(file.size, 5);
        assert!(!file.is_dir);

        let subdir = result.iter().find(|e| e.name == "subdir").unwrap();
        assert!(subdir.is_dir);
        cleanup(&dir);
    }

    #[test]
    fn sort_by_extension() {
        let dir = temp_dir("by_ext");
        fs::write(format!("{}/file.txt", dir), "").unwrap();
        fs::write(format!("{}/file.rs", dir), "").unwrap();
        fs::write(format!("{}/file.go", dir), "").unwrap();

        let opts = LsOptions { sort_by_ext: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["file.go", "file.rs", "file.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn list_single_file() {
        let dir = temp_dir("single_file");
        let file_path = format!("{}/test.txt", dir);
        fs::write(&file_path, "content").unwrap();

        let result = list_directory(&file_path, &LsOptions::default()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "test.txt");
        cleanup(&dir);
    }

    #[test]
    fn extension_of_edge_cases() {
        assert_eq!(extension_of("file.txt"), "txt");
        assert_eq!(extension_of("Makefile"), "");
        assert_eq!(extension_of(".hidden"), "");
        assert_eq!(extension_of("file.tar.gz"), "gz");
    }
}
