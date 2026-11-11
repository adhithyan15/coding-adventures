//! # Directory Scanner and Resource Quota Enforcement (VER08a)
//!
//! Discovers `.rs` files within a target package while enforcing strict resource
//! limits and refusing symlinks. Zero third-party dependencies.

use std::fs;
use std::path::{Path, PathBuf};

/// Directories skipped at the package root only (VER08a §5).  Root-level
/// source files must not reach them through `mod` (`parser.rs`,
/// `check_root_level_mods`), nor Cargo `path` keys (`cargo.rs`).
pub const SKIPPED_ROOT_DIRS: &[&str] = &["target", "node_modules", "_build", "tests", "benches", "examples"];

/// Resource limits (VER08a §5).
pub const MAX_SOURCE_FILES: usize = 10_000;
pub const MAX_FILE_SIZE_BYTES: u64 = 4 * 1024 * 1024; // 4 MiB
pub const MAX_TOTAL_SOURCE_BYTES: u64 = 64 * 1024 * 1024; // 64 MiB
pub const MAX_DIR_DEPTH: usize = 64;

/// Discovered source file metadata and content.
#[derive(Clone, Debug)]
pub struct SourceEntry {
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub content: String,
    pub size_bytes: u64,
}

/// Errors occurring during directory traversal or quota enforcement.
#[derive(Debug)]
pub enum ScanError {
    Io(String),
    SymlinkDetected(String),
    FileLimitExceeded(usize),
    FileSizeLimitExceeded(String, u64),
    TotalSizeLimitExceeded(u64),
    DepthLimitExceeded(usize),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "scanner I/O error: {msg}"),
            Self::SymlinkDetected(p) => write!(f, "symlink refused at '{p}'"),
            Self::FileLimitExceeded(limit) => {
                write!(f, "exceeded maximum source file count ({limit})")
            }
            Self::FileSizeLimitExceeded(p, sz) => {
                write!(f, "file '{p}' exceeds 4 MiB limit ({sz} bytes)")
            }
            Self::TotalSizeLimitExceeded(sz) => {
                write!(f, "total source size exceeds 64 MiB limit ({sz} bytes)")
            }
            Self::DepthLimitExceeded(depth) => {
                write!(f, "exceeded maximum directory depth ({depth})")
            }
        }
    }
}

impl std::error::Error for ScanError {}

/// Recursively discover and read all Rust source files in `package_dir`.
pub fn discover_sources(package_dir: &Path) -> Result<Vec<SourceEntry>, ScanError> {
    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;

    collect_rs_files(package_dir, package_dir, 0, &mut files)?;

    // Deterministic sorting by relative path
    files.sort();

    if files.len() > MAX_SOURCE_FILES {
        return Err(ScanError::FileLimitExceeded(MAX_SOURCE_FILES));
    }

    let mut entries = Vec::with_capacity(files.len());
    for abs_path in files {
        let rel_path = abs_path
            .strip_prefix(package_dir)
            .unwrap_or(&abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let meta = fs::symlink_metadata(&abs_path)
            .map_err(|e| ScanError::Io(format!("{}: {e}", abs_path.display())))?;
        if meta.is_symlink() {
            return Err(ScanError::SymlinkDetected(rel_path));
        }

        let size = meta.len();
        if size > MAX_FILE_SIZE_BYTES {
            return Err(ScanError::FileSizeLimitExceeded(rel_path, size));
        }

        total_bytes += size;
        if total_bytes > MAX_TOTAL_SOURCE_BYTES {
            return Err(ScanError::TotalSizeLimitExceeded(total_bytes));
        }

        let content = fs::read_to_string(&abs_path)
            .map_err(|e| ScanError::Io(format!("{}: {e}", abs_path.display())))?;

        entries.push(SourceEntry {
            rel_path,
            abs_path,
            content,
            size_bytes: size,
        });
    }

    Ok(entries)
}

fn collect_rs_files(
    root: &Path,
    current_dir: &Path,
    depth: usize,
    out: &mut Vec<PathBuf>,
) -> Result<(), ScanError> {
    if depth > MAX_DIR_DEPTH {
        return Err(ScanError::DepthLimitExceeded(MAX_DIR_DEPTH));
    }

    let read_dir = fs::read_dir(current_dir)
        .map_err(|e| ScanError::Io(format!("{}: {e}", current_dir.display())))?;

    for entry_res in read_dir {
        let entry = entry_res.map_err(|e| ScanError::Io(e.to_string()))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| ScanError::Io(format!("{}: {e}", path.display())))?;

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip ignored directories.
        //
        // Hidden directories cannot be named by `mod` (a module name is an
        // identifier), so they are only reachable through `#[path]`, which the
        // parser reports as CAP003.  The build/vendor/test directories are
        // skipped at the package ROOT only: a nested `src/target/mod.rs` is an
        // ordinary module (`mod target;`) and must be scanned.
        if meta.is_dir() {
            let at_root = current_dir == root;
            if name.starts_with('.') || (at_root && SKIPPED_ROOT_DIRS.contains(&name.as_str())) {
                continue;
            }
            collect_rs_files(root, &path, depth + 1, out)?;
        } else if meta.is_symlink() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            return Err(ScanError::SymlinkDetected(rel));
        } else if meta.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            if out.len() >= MAX_SOURCE_FILES {
                return Err(ScanError::FileLimitExceeded(MAX_SOURCE_FILES));
            }
            out.push(path);
        }
    }

    Ok(())
}
