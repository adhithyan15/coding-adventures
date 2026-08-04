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
// We infer a package's language from its directory path using the canonical
// package-parity bucket registry plus the shared `dotnet` program host bucket.
// The package name is "{language}/{dirname}", e.g., "python/logic-gates" or
// "go/directed-graph".

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const DUPLICATE_PACKAGE_IDENTITY: &str = "DUPLICATE_PACKAGE_IDENTITY";

/// Canonical repository buckets understood by package discovery.
///
/// The parity denominator is defined in `package_parity_report.py`. Discovery
/// additionally retains `dotnet` for programs hosted by the shared .NET engine.
pub const DISCOVERY_LANGUAGES: &[&str] = &[
    "csharp",
    "dart",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
    "c",
    "cpp",
    "ocaml",
    "wasm",
    "mosaic",
    "twig",
    "starlark",
    "dotnet",
];

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
    /// Inferred canonical discovery language, or "unknown".
    pub language: String,
}

/// Two or more package directories that normalize to one graph identity.
#[derive(Debug, Eq, PartialEq)]
pub struct DuplicatePackageIdentityError {
    pub code: String,
    pub package: String,
    pub paths: Vec<String>,
}

impl fmt::Display for DuplicatePackageIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: package={} paths={}",
            self.code,
            self.package,
            self.paths.join(",")
        )
    }
}

impl std::error::Error for DuplicatePackageIdentityError {}

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
    "specs",
    "Pods",
    ".dart_tool",
    ".build",
    ".gradle",
    "gradle-build",
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
/// We look for canonical language names as exact path components. For example,
/// "/repo/code/packages/python/logic-gates" yields "python".
fn infer_language(path: &Path) -> String {
    // Convert path to forward-slash form for consistent splitting across platforms.
    let path_str = path.to_string_lossy().replace('\\', "/");
    let parts: Vec<&str> = path_str.split('/').collect();

    for pair in parts.windows(2) {
        if pair[0] == "packages" || pair[0] == "programs" {
            return if DISCOVERY_LANGUAGES.contains(&pair[1]) {
                pair[1].to_string()
            } else {
                "unknown".to_string()
            };
        }
    }
    "unknown".to_string()
}

fn repository_package_path(root: &Path, path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut canonical_start = None;
    for index in 0..parts.len().saturating_sub(1) {
        if parts[index] == "code"
            && (parts[index + 1] == "packages" || parts[index + 1] == "programs")
        {
            canonical_start = Some(index);
        }
    }
    if let Some(index) = canonical_start {
        return parts[index..].join("/");
    }

    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .filter(|relative| !relative.is_empty())
        .or_else(|| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

/// Builds a qualified package name from the language and directory path.
///
/// Programs retain a `programs` identity segment so a library package and a
/// program with the same basename remain distinct graph nodes.
fn infer_package_name(path: &Path, language: &str) -> String {
    let dir_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let components: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    if components
        .windows(2)
        .any(|pair| pair[0] == "code" && pair[1] == "programs")
    {
        return format!("{}/programs/{}", language, dir_name);
    }
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
pub fn discover_packages(root: &Path) -> Result<Vec<Package>, DuplicatePackageIdentityError> {
    let mut packages = Vec::new();
    walk_dirs(root, &mut packages);
    packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.path.cmp(&b.path)));

    let mut index = 0;
    while index < packages.len() {
        let mut end = index + 1;
        while end < packages.len() && packages[end].name == packages[index].name {
            end += 1;
        }
        if end - index > 1 {
            let paths = packages[index..end]
                .iter()
                .map(|package| repository_package_path(root, &package.path))
                .collect();
            return Err(DuplicatePackageIdentityError {
                code: DUPLICATE_PACKAGE_IDENTITY.to_string(),
                package: packages[index].name.clone(),
                paths,
            });
        }
        index = end;
    }

    Ok(packages)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    struct DiscoveryFixture {
        workspace: FixtureWorkspace,
        expected: ExpectedDiscovery,
    }

    #[derive(Deserialize)]
    struct FixtureWorkspace {
        files: Vec<FixtureFile>,
    }

    #[derive(Deserialize)]
    struct FixtureFile {
        path: String,
        content_utf8: String,
    }

    #[derive(Deserialize)]
    struct ExpectedDiscovery {
        result: ExpectedResult,
        diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    struct ExpectedResult {
        #[serde(default)]
        packages: Vec<ExpectedPackage>,
    }

    #[derive(Deserialize)]
    struct ExpectedPackage {
        language: String,
        name: String,
        rel_path: String,
    }

    #[derive(Deserialize)]
    struct ExpectedDiagnostic {
        code: String,
        path: String,
        package: String,
        details: ExpectedDiagnosticDetails,
    }

    #[derive(Deserialize)]
    struct ExpectedDiagnosticDetails {
        paths: Vec<String>,
    }

    fn load_discovery_fixture(name: &str) -> DiscoveryFixture {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/fixtures/build-tool-v1/cases")
            .join(name);
        let data = fs::read(&path)
            .unwrap_or_else(|error| panic!("read shared fixture {}: {error}", path.display()));
        serde_json::from_slice(&data)
            .unwrap_or_else(|error| panic!("decode shared fixture {}: {error}", path.display()))
    }

    fn materialize_discovery_fixture(fixture: &DiscoveryFixture, case_name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "build_tool_rust_discovery_{case_name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for file in &fixture.workspace.files {
            let path = root.join(Path::new(&file.path));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &file.content_utf8).unwrap();
        }
        root
    }

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
    fn test_infer_package_name_preserves_program_identity() {
        let path = Path::new("/repo/code/programs/elixir/grammar_tools");
        assert_eq!(
            infer_package_name(path, "elixir"),
            "elixir/programs/grammar_tools"
        );
    }

    #[test]
    fn test_language_registry_conformance_fixture() {
        let fixture = load_discovery_fixture("discovery-language-registry.json");
        let root = materialize_discovery_fixture(&fixture, "language_registry");
        let packages = discover_packages(&root.join("code"))
            .expect("the canonical language registry fixture must discover");
        let actual: Vec<(String, String, String)> = packages
            .iter()
            .map(|package| {
                (
                    package.name.clone(),
                    package.language.clone(),
                    package
                        .path
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                )
            })
            .collect();
        let expected: Vec<(String, String, String)> = fixture
            .expected
            .result
            .packages
            .iter()
            .map(|package| {
                (
                    package.name.clone(),
                    package.language.clone(),
                    package.rel_path.clone(),
                )
            })
            .collect();
        assert_eq!(actual, expected);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_duplicate_identity_conformance_fixture() {
        let fixture = load_discovery_fixture("discovery-duplicate-identity.json");
        let root = materialize_discovery_fixture(&fixture, "duplicate_identity");
        let error = discover_packages(&root.join("code"))
            .expect_err("duplicate qualified identities must fail closed");
        let diagnostic = &fixture.expected.diagnostics[0];
        assert_eq!(error.code, diagnostic.code);
        assert_eq!(error.package, diagnostic.package);
        assert_eq!(error.paths[0], diagnostic.path);
        assert_eq!(error.paths, diagnostic.details.paths);
        let _ = fs::remove_dir_all(root);
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

        let packages = discover_packages(&dir).expect("fixture identities are unique");
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
