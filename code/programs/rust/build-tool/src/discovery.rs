// Package discovery — walks a monorepo directory tree to discover packages.
//
// # How package discovery works
//
// A monorepo can contain hundreds of packages across multiple languages. The
// build system discovers them by recursively walking the directory tree and
// looking for BUILD files. Any directory containing a BUILD file is a package.
//
// The walk is recursive. Starting from the root:
//
//  1. If the current directory's name is in the skip list, ignore it entirely.
//  2. If the current directory has a BUILD file, it is a package. Register it
//     and stop — we don't recurse into packages.
//  3. Otherwise, list all subdirectories and recurse into each one.
//
// This is the same approach used by Bazel, Buck, and Pants. No configuration
// files are needed to route the walk — the presence of a BUILD file is
// sufficient to identify a package.
//
// # Skip list
//
// Certain directories are known to never contain packages: .git, .venv,
// node_modules, __pycache__, etc. The skip list prevents the walker from
// descending into these directories, keeping discovery fast even in large
// repos with deep dependency trees.
//
// # Platform-specific BUILD files
//
// On macOS, if BUILD_mac exists in a directory, we use it instead of BUILD.
// On Linux, BUILD_linux takes precedence. This allows platform-specific build
// commands (e.g., different compiler flags or test runners).
//
// # Language inference
//
// We infer a package's language from its directory path. If the path contains
// "python", "ruby", "go", or "rust" as a component under "packages" or
// "programs", that is the language. The package name is "{language}/{dirname}",
// e.g., "python/logic-gates" or "go/directed-graph".

use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Represents a discovered package in the monorepo. Each package has a
/// qualified name (like "python/logic-gates"), an absolute path on disk,
/// a list of build commands from its BUILD file, and an inferred language.
#[derive(Debug, Clone)]
pub struct Package {
    /// Qualified name, e.g. "python/logic-gates".
    pub name: String,
    /// Absolute path to the package directory.
    pub path: PathBuf,
    /// Lines from the BUILD file (commands to execute).
    pub build_commands: Vec<String>,
    /// Inferred language: "python", "ruby", "go", "rust", or "unknown".
    pub language: String,
}

// ---------------------------------------------------------------------------
// Skip list
// ---------------------------------------------------------------------------

/// Directory names that should never be traversed during discovery.
/// These are known to contain non-source files (caches, dependencies,
/// build artifacts) that would waste time to scan and could never
/// contain valid packages.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "target",
    ".claude",
    "Pods",
];

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Reads a file and returns non-blank, non-comment lines.
///
/// Blank lines and lines starting with '#' are stripped out. Leading and
/// trailing whitespace is removed from each line. If the file does not
/// exist, an empty Vec is returned (not an error — a missing file
/// simply means "nothing to see here").
pub fn read_lines(path: &Path) -> Vec<String> {
    let data = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    data.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Inspects the directory path to determine the programming language.
/// We look for known language names ("python", "ruby", "go", "rust")
/// as path components. For example, "/repo/code/packages/python/logic-gates"
/// yields "python".
fn infer_language(path: &Path) -> String {
    // Convert path to forward-slash form for consistent splitting across platforms.
    let path_str = path.to_string_lossy().replace('\\', "/");
    let parts: Vec<&str> = path_str.split('/').collect();

    for lang in &[
        "python",
        "ruby",
        "go",
        "rust",
        "typescript",
        "elixir",
        "lua",
        "perl",
        "swift",
        "haskell",
        "wasm",
        "csharp",
        "fsharp",
        "dotnet",
    ] {
        for part in &parts {
            if part == lang {
                return lang.to_string();
            }
        }
    }
    "unknown".to_string()
}

/// Builds a qualified package name like "python/logic-gates" from the
/// language and the directory's basename.
fn infer_package_name(path: &Path, language: &str) -> String {
    let dir_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("{}/{}", language, dir_name)
}

/// Returns the path to the appropriate BUILD file for the current
/// platform, or None if none exists.
///
/// Priority (most specific wins):
///  1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
///  2. Shared: BUILD_mac_and_linux (macOS or Linux — for Unix-like systems)
///  3. Generic: BUILD (all platforms)
///  4. None if no BUILD file exists
///
/// This layering lets packages provide Windows-specific build commands via
/// BUILD_windows while sharing a single BUILD_mac_and_linux for the common
/// Unix case, falling back to BUILD when no platform differences exist.
fn get_build_file(directory: &Path) -> Option<PathBuf> {
    get_build_file_for_os(directory, std::env::consts::OS)
}

/// Shared implementation for both runtime and test use. The `os` parameter
/// should be "macos", "darwin", "linux", or "windows".
fn get_build_file_for_os(directory: &Path, os: &str) -> Option<PathBuf> {
    // Step 1: Check for the most specific platform file.
    if os == "macos" || os == "darwin" {
        let platform_build = directory.join("BUILD_mac");
        if platform_build.is_file() {
            return Some(platform_build);
        }
    }

    if os == "linux" {
        let platform_build = directory.join("BUILD_linux");
        if platform_build.is_file() {
            return Some(platform_build);
        }
    }

    if os == "windows" {
        let platform_build = directory.join("BUILD_windows");
        if platform_build.is_file() {
            return Some(platform_build);
        }
    }

    // Step 2: Check for the shared Unix file (macOS + Linux).
    if os == "macos" || os == "darwin" || os == "linux" {
        let shared_build = directory.join("BUILD_mac_and_linux");
        if shared_build.is_file() {
            return Some(shared_build);
        }
    }

    // Step 3: Fall back to the generic BUILD file.
    let generic_build = directory.join("BUILD");
    if generic_build.is_file() {
        return Some(generic_build);
    }

    None
}

/// Like `get_build_file` but accepts an explicit OS name for testing
/// platform-specific behavior without running on that platform.
#[cfg(test)]
pub fn get_build_file_for_platform(directory: &Path, os: &str) -> Option<PathBuf> {
    get_build_file_for_os(directory, os)
}

// ---------------------------------------------------------------------------
// Walk algorithm
// ---------------------------------------------------------------------------

/// Recursively descends into subdirectories, collecting packages that have
/// BUILD files. This is the heart of the discovery algorithm.
///
/// The walk uses the skip list to avoid descending into directories that are
/// known to contain non-source files (caches, dependencies, build artifacts).
///
/// The recursion stops at BUILD files: once we find a package, we don't
/// look inside it for sub-packages. This keeps the model simple — a
/// package is a leaf in the directory tree.
fn walk_dirs(directory: &Path, packages: &mut Vec<Package>) {
    // Check if this directory's name is in the skip list.
    if let Some(dir_name) = directory.file_name() {
        let name = dir_name.to_string_lossy();
        if SKIP_DIRS.contains(&name.as_ref()) {
            return;
        }
    }

    if let Some(build_file) = get_build_file(directory) {
        // This directory is a package. Read the BUILD commands and register it.
        let commands = read_lines(&build_file);
        let language = infer_language(directory);
        let name = infer_package_name(directory, &language);

        packages.push(Package {
            name,
            path: directory.to_path_buf(),
            build_commands: commands,
            language,
        });
        return; // Don't recurse into packages.
    }

    // Not a package — list all subdirectories and recurse into each one.
    let entries = match fs::read_dir(directory) {
        Ok(e) => e,
        Err(_) => return,
    };

    // Collect and sort entries for deterministic ordering across platforms.
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    dirs.sort();

    for subdir in dirs {
        walk_dirs(&subdir, packages);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Recursively walks the directory tree starting from root, collecting
/// packages with BUILD files. The returned list is sorted by package
/// name for deterministic output.
///
/// This is the main entry point for the discovery module. The root
/// parameter should typically be the "code/" directory inside the repo.
pub fn discover_packages(root: &Path) -> Vec<Package> {
    let mut packages = Vec::new();
    walk_dirs(root, &mut packages);
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_infer_language() {
        let path = Path::new("/repo/code/packages/python/logic-gates");
        assert_eq!(infer_language(path), "python");

        let path = Path::new("/repo/code/programs/go/build-tool");
        assert_eq!(infer_language(path), "go");

        let path = Path::new("/repo/code/packages/rust/parser");
        assert_eq!(infer_language(path), "rust");

        let path = Path::new("/repo/unknown-dir");
        assert_eq!(infer_language(path), "unknown");
    }

    #[test]
    fn test_infer_package_name() {
        let path = Path::new("/repo/code/packages/python/logic-gates");
        assert_eq!(infer_package_name(path, "python"), "python/logic-gates");
    }

    #[test]
    fn test_read_lines_filters_comments_and_blanks() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_read_lines_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let file = dir.join("BUILD");
        fs::write(
            &file,
            "# This is a comment\n\npip install .\n  pytest  \n# Another comment\n",
        )
        .unwrap();

        let lines = read_lines(&file);
        assert_eq!(lines, vec!["pip install .", "pytest"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_lines_missing_file() {
        let lines = read_lines(Path::new("/nonexistent/file"));
        assert!(lines.is_empty());
    }

    #[test]
    fn test_discover_packages_with_temp_dir() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_discover_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Create two packages.
        let py_pkg = dir.join("packages/python/logic-gates");
        fs::create_dir_all(&py_pkg).unwrap();
        fs::write(py_pkg.join("BUILD"), "pytest\n").unwrap();

        let go_pkg = dir.join("packages/go/directed-graph");
        fs::create_dir_all(&go_pkg).unwrap();
        fs::write(go_pkg.join("BUILD"), "go test ./...\n").unwrap();

        // Create a .git dir that should be skipped.
        let git_dir = dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("BUILD"), "nope").unwrap();

        let packages = discover_packages(&dir);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "go/directed-graph");
        assert_eq!(packages[1].name, "python/logic-gates");
        assert_eq!(packages[1].build_commands, vec!["pytest"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_build_file() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_platform_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Create both generic and platform-specific BUILD files.
        fs::write(dir.join("BUILD"), "generic command").unwrap();
        fs::write(dir.join("BUILD_mac"), "mac command").unwrap();
        fs::write(dir.join("BUILD_linux"), "linux command").unwrap();

        // Test macOS priority.
        let result = get_build_file_for_platform(&dir, "darwin");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_mac"));

        // Test Linux priority.
        let result = get_build_file_for_platform(&dir, "linux");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_linux"));

        // Test fallback to generic BUILD when no windows-specific file.
        let result = get_build_file_for_platform(&dir, "windows");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_windows_preferred() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_win_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("BUILD"), "generic").unwrap();
        fs::write(dir.join("BUILD_windows"), "windows").unwrap();

        // Windows should prefer BUILD_windows.
        let result = get_build_file_for_platform(&dir, "windows");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_windows"));

        // macOS should NOT use BUILD_windows — falls back to BUILD.
        let result = get_build_file_for_platform(&dir, "darwin");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with("BUILD") && !path.to_string_lossy().contains("windows"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_mac_and_linux() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_maclinux_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("BUILD"), "generic").unwrap();
        fs::write(dir.join("BUILD_mac_and_linux"), "unix").unwrap();

        // macOS should use BUILD_mac_and_linux.
        let result = get_build_file_for_platform(&dir, "darwin");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_mac_and_linux"));

        // Linux should use BUILD_mac_and_linux.
        let result = get_build_file_for_platform(&dir, "linux");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_mac_and_linux"));

        // Windows should NOT use BUILD_mac_and_linux — falls back to BUILD.
        let result = get_build_file_for_platform(&dir, "windows");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with("BUILD") && !path.to_string_lossy().contains("mac_and_linux"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_mac_overrides_mac_and_linux() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_override_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("BUILD"), "generic").unwrap();
        fs::write(dir.join("BUILD_mac"), "mac").unwrap();
        fs::write(dir.join("BUILD_mac_and_linux"), "unix").unwrap();

        // BUILD_mac is more specific than BUILD_mac_and_linux.
        let result = get_build_file_for_platform(&dir, "darwin");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("BUILD_mac"));

        let _ = fs::remove_dir_all(&dir);
    }
}
