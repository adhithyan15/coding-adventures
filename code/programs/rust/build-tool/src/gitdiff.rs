// Git-based change detection for the build tool.
//
// ==========================================================================
// Chapter 1: Git as the Source of Truth
// ==========================================================================
//
// Instead of maintaining a cache file, this module uses git diff to determine
// which files changed between the current branch and a base ref (typically
// origin/main). Changed files are mapped to packages, then the dependency
// graph's affected_nodes() finds everything that needs rebuilding.
//
// This is the DEFAULT change detection mode. Git is the source of truth.
//
// ==========================================================================
// Chapter 2: Three-dot vs Two-dot Diff
// ==========================================================================
//
// We prefer three-dot diff (`base...HEAD`) because it shows changes since
// the merge base — exactly what we want for PR builds. If three-dot fails
// (e.g., the remote hasn't been fetched), we fall back to two-dot diff.
//
// ==========================================================================
// Chapter 3: Strict Starlark-Aware Filtering
// ==========================================================================
//
// Not every changed file under a package directory is build-relevant.
// Consider a package at `code/packages/python/logic-gates/`. If someone
// edits `README.md` or `CHANGELOG.md` inside that directory, we do NOT
// want to trigger a rebuild — those are documentation files that have no
// effect on the build output.
//
// Our filtering rules:
//
// | File type              | Included? | Rationale                          |
// |------------------------|-----------|------------------------------------|
// | BUILD, BUILD_*         | Yes       | Build definition changed           |
// | .py, .rb, .go, .rs    | Yes       | Source code changed                |
// | .toml, .cfg            | Yes       | Config that affects build          |
// | .gemspec               | Yes       | Ruby package manifest              |
// | go.mod, go.sum         | Yes       | Go dependency lock                 |
// | Cargo.lock             | Yes       | Rust dependency lock               |
// | Gemfile, Rakefile      | Yes       | Ruby build config                  |
// | package.json           | Yes       | Node.js/TS package manifest        |
// | .ts, .tsx, .js, .jsx   | Yes       | TypeScript/JavaScript source       |
// | .ex, .exs              | Yes       | Elixir source                      |
// | .star                  | Yes       | Starlark rule definitions          |
// | .md, .txt, .yml, etc.  | No        | Documentation, not build-relevant  |
//
// This strict filtering prevents spurious rebuilds when only docs change,
// which is a common source of CI waste in monorepos.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use crate::discovery::Package;

// ---------------------------------------------------------------------------
// Build-relevant file extensions and names
// ---------------------------------------------------------------------------

/// File extensions that are considered build-relevant across all languages.
///
/// A changed file is only mapped to a package if it has one of these
/// extensions OR matches one of the special filenames below. This prevents
/// documentation-only changes from triggering rebuilds.
const BUILD_RELEVANT_EXTENSIONS: &[&str] = &[
    // Python
    ".py", ".pyi", ".toml", ".cfg",
    // Ruby
    ".rb", ".gemspec",
    // Go
    ".go",
    // Rust
    ".rs",
    // TypeScript / JavaScript
    ".ts", ".tsx", ".js", ".jsx",
    // Elixir
    ".ex", ".exs",
    // Starlark BUILD rules
    ".star",
];

/// Filenames (exact matches, no extension check) that are build-relevant.
///
/// These files affect the build even though their extensions might not
/// be in the extension list above.
const BUILD_RELEVANT_FILENAMES: &[&str] = &[
    "BUILD",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_windows",
    "BUILD_mac_and_linux",
    "Gemfile",
    "Gemfile.lock",
    "Rakefile",
    "go.mod",
    "go.sum",
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "mix.exs",
    "mix.lock",
    "pyproject.toml",
    "setup.cfg",
    "setup.py",
];

/// Determines whether a file path is build-relevant based on its
/// extension or filename.
///
/// This is the gatekeeper function for strict filtering. Only files that
/// pass this check will be considered when mapping changed files to
/// packages.
///
/// # Examples
///
/// ```text
/// is_build_relevant("src/gates.py")          -> true  (extension .py)
/// is_build_relevant("BUILD")                 -> true  (special filename)
/// is_build_relevant("go.mod")                -> true  (special filename)
/// is_build_relevant("README.md")             -> false (documentation)
/// is_build_relevant("CHANGELOG.md")          -> false (documentation)
/// is_build_relevant(".github/workflows/ci.yml") -> false (CI config)
/// ```
fn is_build_relevant(file_path: &str) -> bool {
    // Extract the filename (last component after the final `/`).
    let filename = file_path.rsplit('/').next().unwrap_or(file_path);

    // Check special filenames first (exact match).
    if BUILD_RELEVANT_FILENAMES.contains(&filename) {
        return true;
    }

    // Check file extension.
    for ext in BUILD_RELEVANT_EXTENSIONS {
        if filename.ends_with(ext) {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Git diff
// ---------------------------------------------------------------------------

/// Runs `git diff --name-only <base>...HEAD` and returns the list of
/// changed file paths relative to the repo root.
///
/// Uses three-dot diff which shows changes since the merge base — exactly
/// what we want for PR builds. Falls back to two-dot diff if three-dot fails.
pub fn get_changed_files(repo_root: &Path, diff_base: &str) -> Vec<String> {
    // Try three-dot diff first (merge base).
    let three_dot = format!("{}...HEAD", diff_base);
    let output = Command::new("git")
        .args(["diff", "--name-only", &three_dot])
        .current_dir(repo_root)
        .output();

    let out = match output {
        Ok(o) if o.status.success() => o.stdout,
        _ => {
            // Fallback: two-dot diff.
            let output = Command::new("git")
                .args(["diff", "--name-only", diff_base, "HEAD"])
                .current_dir(repo_root)
                .output();
            match output {
                Ok(o) if o.status.success() => o.stdout,
                _ => return Vec::new(),
            }
        }
    };

    let text = String::from_utf8_lossy(&out);
    text.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// File-to-package mapping
// ---------------------------------------------------------------------------

/// Maps changed file paths to package names with strict Starlark-aware
/// filtering.
///
/// A file is mapped to a package only if BOTH conditions are met:
///   1. The file's path starts with the package's directory prefix.
///   2. The file is build-relevant (source code, config, or BUILD file).
///
/// This two-layer check prevents documentation changes from triggering
/// rebuilds while still catching all meaningful source changes.
///
/// # Arguments
///
/// * `changed_files` — Paths from `git diff --name-only`, relative to repo root.
/// * `packages` — All discovered packages.
/// * `repo_root` — Absolute path to the repository root.
///
/// # Returns
///
/// A set of package names whose source files changed.
pub fn map_files_to_packages(
    changed_files: &[String],
    packages: &[Package],
    repo_root: &Path,
) -> HashSet<String> {
    let mut changed = HashSet::new();

    // Build relative path lookup for each package.
    //
    // We precompute the relative path for each package once, rather than
    // recomputing it for every changed file. This is O(packages) setup
    // for O(files * packages) matching — the inner loop is just string
    // prefix comparison.
    struct PkgInfo {
        name: String,
        rel_path: String,
    }

    let mut pkg_paths: Vec<PkgInfo> = Vec::new();
    for pkg in packages {
        if let Ok(rel) = pkg.path.strip_prefix(repo_root) {
            // Normalize to forward slashes for cross-platform comparison.
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            pkg_paths.push(PkgInfo {
                name: pkg.name.clone(),
                rel_path: rel_str,
            });
        }
    }

    for f in changed_files {
        // Normalize the changed file path to forward slashes too.
        let f_normalized = f.replace('\\', "/");

        // Strict filter: skip non-build-relevant files immediately.
        //
        // This is the key improvement over the naive approach. If someone
        // edits README.md inside a package directory, we skip it here
        // rather than triggering a rebuild.
        if !is_build_relevant(&f_normalized) {
            continue;
        }

        for pkg in &pkg_paths {
            let prefix_with_slash = format!("{}/", pkg.rel_path);
            if f_normalized.starts_with(&prefix_with_slash) || f_normalized == pkg.rel_path {
                changed.insert(pkg.name.clone());
                break;
            }
        }
    }

    changed
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -----------------------------------------------------------------------
    // is_build_relevant tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_relevant_python_source() {
        assert!(is_build_relevant("src/gates.py"));
        assert!(is_build_relevant("deep/nested/module.py"));
    }

    #[test]
    fn test_build_relevant_rust_source() {
        assert!(is_build_relevant("src/main.rs"));
    }

    #[test]
    fn test_build_relevant_go_source() {
        assert!(is_build_relevant("graph.go"));
    }

    #[test]
    fn test_build_relevant_typescript_source() {
        assert!(is_build_relevant("src/index.ts"));
        assert!(is_build_relevant("component.tsx"));
    }

    #[test]
    fn test_build_relevant_elixir_source() {
        assert!(is_build_relevant("lib/app.ex"));
        assert!(is_build_relevant("test/app_test.exs"));
    }

    #[test]
    fn test_build_relevant_starlark_rules() {
        assert!(is_build_relevant("rules/python_library.star"));
    }

    #[test]
    fn test_build_relevant_build_files() {
        assert!(is_build_relevant("BUILD"));
        assert!(is_build_relevant("code/packages/python/logic-gates/BUILD"));
        assert!(is_build_relevant("BUILD_mac"));
        assert!(is_build_relevant("BUILD_linux"));
        assert!(is_build_relevant("BUILD_windows"));
        assert!(is_build_relevant("BUILD_mac_and_linux"));
    }

    #[test]
    fn test_build_relevant_special_filenames() {
        assert!(is_build_relevant("Gemfile"));
        assert!(is_build_relevant("Rakefile"));
        assert!(is_build_relevant("go.mod"));
        assert!(is_build_relevant("go.sum"));
        assert!(is_build_relevant("Cargo.toml"));
        assert!(is_build_relevant("Cargo.lock"));
        assert!(is_build_relevant("package.json"));
        assert!(is_build_relevant("mix.exs"));
        assert!(is_build_relevant("pyproject.toml"));
    }

    #[test]
    fn test_not_build_relevant_docs() {
        assert!(!is_build_relevant("README.md"));
        assert!(!is_build_relevant("CHANGELOG.md"));
        assert!(!is_build_relevant("docs/guide.md"));
    }

    #[test]
    fn test_not_build_relevant_ci_config() {
        assert!(!is_build_relevant(".github/workflows/ci.yml"));
        assert!(!is_build_relevant(".gitignore"));
    }

    #[test]
    fn test_not_build_relevant_misc() {
        assert!(!is_build_relevant("LICENSE"));
        assert!(!is_build_relevant("notes.txt"));
        assert!(!is_build_relevant("diagram.png"));
    }

    // -----------------------------------------------------------------------
    // map_files_to_packages tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_files_to_packages() {
        let repo_root = PathBuf::from("/repo");
        let packages = vec![
            Package {
                name: "python/logic-gates".to_string(),
                path: PathBuf::from("/repo/code/packages/python/logic-gates"),
                build_commands: vec![],
                language: "python".to_string(),
            },
            Package {
                name: "go/directed-graph".to_string(),
                path: PathBuf::from("/repo/code/packages/go/directed-graph"),
                build_commands: vec![],
                language: "go".to_string(),
            },
        ];

        let changed_files = vec![
            "code/packages/python/logic-gates/src/gates.py".to_string(),
            "README.md".to_string(), // Not in any package + not build-relevant.
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.contains("python/logic-gates"));
        assert!(!result.contains("go/directed-graph"));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_map_files_to_packages_empty() {
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let changed_files: Vec<String> = vec![];
        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_files_no_matching_package() {
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let changed_files = vec!["docs/README.md".to_string()];
        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_files_strict_filters_readme_in_package() {
        // This is the KEY test for strict filtering.
        // A README.md INSIDE a package directory should NOT trigger a rebuild.
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let changed_files = vec![
            "code/packages/python/logic-gates/README.md".to_string(),
            "code/packages/python/logic-gates/CHANGELOG.md".to_string(),
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(
            result.is_empty(),
            "README.md and CHANGELOG.md should not trigger a rebuild"
        );
    }

    #[test]
    fn test_map_files_strict_allows_source_in_package() {
        // Source files INSIDE a package SHOULD trigger a rebuild.
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let changed_files = vec![
            "code/packages/python/logic-gates/src/gates.py".to_string(),
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.contains("python/logic-gates"));
    }

    #[test]
    fn test_map_files_strict_allows_build_file() {
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let changed_files = vec![
            "code/packages/python/logic-gates/BUILD".to_string(),
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.contains("python/logic-gates"));
    }

    #[test]
    fn test_map_files_mixed_relevant_and_irrelevant() {
        // Mix of build-relevant and non-relevant files in the same package.
        // Only the relevant ones should cause the package to appear.
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "go/directed-graph".to_string(),
            path: PathBuf::from("/repo/code/packages/go/directed-graph"),
            build_commands: vec![],
            language: "go".to_string(),
        }];

        // Only the .go file is relevant — but that's enough.
        let changed_files = vec![
            "code/packages/go/directed-graph/README.md".to_string(),
            "code/packages/go/directed-graph/graph.go".to_string(),
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.contains("go/directed-graph"));
    }

    #[test]
    fn test_map_files_only_irrelevant_in_package() {
        // Only non-relevant files changed inside the package.
        let repo_root = PathBuf::from("/repo");
        let packages = vec![Package {
            name: "go/directed-graph".to_string(),
            path: PathBuf::from("/repo/code/packages/go/directed-graph"),
            build_commands: vec![],
            language: "go".to_string(),
        }];

        let changed_files = vec![
            "code/packages/go/directed-graph/README.md".to_string(),
            "code/packages/go/directed-graph/CHANGELOG.md".to_string(),
            "code/packages/go/directed-graph/docs/design.txt".to_string(),
        ];

        let result = map_files_to_packages(&changed_files, &packages, &repo_root);
        assert!(result.is_empty());
    }
}
