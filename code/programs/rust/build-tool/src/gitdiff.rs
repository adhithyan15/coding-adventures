// Git-based change detection for the build tool.
//
// Instead of maintaining a cache file, this module uses git diff to determine
// which files changed between the current branch and a base ref (typically
// origin/main). Changed files are mapped to packages, then the dependency
// graph's affected_nodes() finds everything that needs rebuilding.
//
// This is the DEFAULT change detection mode. Git is the source of truth.
//
// # Three-dot vs two-dot diff
//
// We prefer three-dot diff (`base...HEAD`) because it shows changes since
// the merge base — exactly what we want for PR builds. If three-dot fails
// (e.g., the remote hasn't been fetched), we fall back to two-dot diff.

use std::path::Path;
use std::process::Command;

use crate::discovery::Package;

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

/// Maps changed file paths to package names.
/// A file belongs to a package if its path starts with the package's
/// directory path relative to the repo root.
pub fn map_files_to_packages(
    changed_files: &[String],
    packages: &[Package],
    repo_root: &Path,
) -> std::collections::HashSet<String> {
    let mut changed = std::collections::HashSet::new();

    // Build relative path lookup for each package.
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
            "README.md".to_string(), // Not in any package.
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
}
