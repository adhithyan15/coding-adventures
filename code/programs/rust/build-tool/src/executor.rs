// Parallel build execution engine.
//
// # Parallel execution by levels
//
// The key insight of the build system is that not all packages depend on
// each other. The dependency graph can be partitioned into "levels" where
// packages within the same level have no dependencies on each other. These
// can safely run in parallel.
//
// For example, in a diamond dependency graph A->B, A->C, B->D, C->D:
//
//   Level 0: [A]     — no dependencies, build first
//   Level 1: [B, C]  — depend only on A, can run in parallel
//   Level 2: [D]     — depends on B and C, build last
//
// # Rayon's advantage
//
// Where the Go implementation uses goroutines with a semaphore-buffered
// channel, we use Rayon's work-stealing thread pool. Rayon automatically
// manages a pool of OS threads (typically one per CPU core) and distributes
// work items across them. This is more idiomatic Rust and avoids manual
// thread management.
//
// The pattern: for each level, we use rayon::scope to spawn parallel tasks
// for all packages in that level. Rayon limits concurrency to its thread
// pool size (configurable via --jobs).
//
// # Failure propagation
//
// If a package fails, all its transitive dependents are marked "dep-skipped".
// There is no point building something whose dependency is broken.

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;

use crate::cache::BuildCache;
use crate::discovery::Package;
use crate::graph::Graph;
use crate::hasher::collect_transitive_predecessors;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Holds the outcome of building a single package.
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// Qualified name, e.g. "python/logic-gates".
    pub package_name: String,
    /// "built", "failed", "skipped", "dep-skipped", "would-build".
    pub status: String,
    /// Wall-clock seconds spent building.
    pub duration: f64,
    /// Combined stdout from all BUILD commands.
    pub stdout: String,
    /// Combined stderr from all BUILD commands.
    pub stderr: String,
    /// Exit code of the last failing command, or 0.
    pub return_code: i32,
}

// ---------------------------------------------------------------------------
// Build execution
// ---------------------------------------------------------------------------

/// Executes all BUILD commands for a single package.
///
/// Commands are run sequentially — each must succeed before the next starts.
/// This is because BUILD files are scripts: later commands may depend on
/// earlier ones (e.g., "install dependencies" before "run tests").
///
/// We use std::process::Command with shell execution (sh -c on Unix,
/// cmd /c on Windows) so that BUILD commands can use shell features like
/// pipes, redirects, and environment variables.
fn run_package_build(pkg: &Package) -> BuildResult {
    let start = Instant::now();
    let mut all_stdout = Vec::new();
    let mut all_stderr = Vec::new();

    for command in &pkg.build_commands {
        // Use platform-appropriate shell invocation.
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .current_dir(&pkg.path)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .current_dir(&pkg.path)
                .output()
        };

        match output {
            Ok(out) => {
                all_stdout.push(String::from_utf8_lossy(&out.stdout).to_string());
                all_stderr.push(String::from_utf8_lossy(&out.stderr).to_string());

                if !out.status.success() {
                    let exit_code = out.status.code().unwrap_or(1);
                    return BuildResult {
                        package_name: pkg.name.clone(),
                        status: "failed".to_string(),
                        duration: start.elapsed().as_secs_f64(),
                        stdout: all_stdout.join(""),
                        stderr: all_stderr.join(""),
                        return_code: exit_code,
                    };
                }
            }
            Err(e) => {
                all_stderr.push(format!("Failed to execute command: {}", e));
                return BuildResult {
                    package_name: pkg.name.clone(),
                    status: "failed".to_string(),
                    duration: start.elapsed().as_secs_f64(),
                    stdout: all_stdout.join(""),
                    stderr: all_stderr.join(""),
                    return_code: 1,
                };
            }
        }
    }

    BuildResult {
        package_name: pkg.name.clone(),
        status: "built".to_string(),
        duration: start.elapsed().as_secs_f64(),
        stdout: all_stdout.join(""),
        stderr: all_stderr.join(""),
        return_code: 0,
    }
}

/// Runs BUILD commands for packages respecting dependency order.
///
/// This is the main orchestrator. It:
///  1. Gets independent_groups from the dependency graph
///  2. For each level, determines which packages need building
///  3. Skips packages whose deps failed ("dep-skipped")
///  4. Skips packages whose hashes haven't changed ("skipped")
///  5. In dry-run mode, marks packages as "would-build"
///  6. Otherwise, launches parallel builds using rayon
///  7. Updates the cache after each build
///
/// The function returns a map from package name to BuildResult.
pub fn execute_builds(
    packages: &[Package],
    graph: &Graph,
    build_cache: &BuildCache,
    package_hashes: &HashMap<String, String>,
    deps_hashes: &HashMap<String, String>,
    force: bool,
    dry_run: bool,
    max_jobs: usize,
    affected_set: Option<&HashSet<String>>,
) -> HashMap<String, BuildResult> {
    // Build a lookup from name to Package for quick access.
    let pkg_by_name: HashMap<&str, &Package> = packages
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    // Get the parallel execution levels from the dependency graph.
    let groups = match graph.independent_groups() {
        Ok(g) => g,
        Err(e) => {
            // Cycle detected — return an error result for all packages.
            let mut results = HashMap::new();
            for pkg in packages {
                results.insert(
                    pkg.name.clone(),
                    BuildResult {
                        package_name: pkg.name.clone(),
                        status: "failed".to_string(),
                        duration: 0.0,
                        stdout: String::new(),
                        stderr: format!("cycle detected in dependency graph: {}", e),
                        return_code: 1,
                    },
                );
            }
            return results;
        }
    };

    // Configure rayon's thread pool to respect --jobs.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(max_jobs)
        .build()
        .unwrap_or_else(|_| {
            // Fallback: use global pool.
            rayon::ThreadPoolBuilder::new().build().unwrap()
        });

    let results: Mutex<HashMap<String, BuildResult>> = Mutex::new(HashMap::new());
    let failed_packages: Mutex<HashSet<String>> = Mutex::new(HashSet::new());

    for level in &groups {
        // Determine what to build in this level.
        // We separate the decision phase (sequential) from the build phase (parallel).
        let mut to_build: Vec<&Package> = Vec::new();

        for name in level {
            let pkg = match pkg_by_name.get(name.as_str()) {
                Some(p) => *p,
                None => continue,
            };

            // Check if any dependency of this package failed.
            let preds = collect_transitive_predecessors(name, graph);
            let dep_failed = {
                let failed = failed_packages.lock().unwrap();
                preds.iter().any(|dep| failed.contains(dep))
            };

            if dep_failed {
                results.lock().unwrap().insert(
                    name.clone(),
                    BuildResult {
                        package_name: name.clone(),
                        status: "dep-skipped".to_string(),
                        duration: 0.0,
                        stdout: String::new(),
                        stderr: String::new(),
                        return_code: 0,
                    },
                );
                continue;
            }

            // Check if the package is in the affected set (git-diff mode).
            // If affected_set is Some, it takes priority over cache.
            if let Some(affected) = affected_set {
                if !affected.contains(name) {
                    results.lock().unwrap().insert(
                        name.clone(),
                        BuildResult {
                            package_name: name.clone(),
                            status: "skipped".to_string(),
                            duration: 0.0,
                            stdout: String::new(),
                            stderr: String::new(),
                            return_code: 0,
                        },
                    );
                    continue;
                }
            }

            // Check if the package needs building (cache fallback).
            let pkg_hash = package_hashes.get(name.as_str()).map(|s| s.as_str()).unwrap_or("");
            let dep_hash = deps_hashes.get(name.as_str()).map(|s| s.as_str()).unwrap_or("");

            if affected_set.is_none() && !force && !build_cache.needs_build(name, pkg_hash, dep_hash) {
                results.lock().unwrap().insert(
                    name.clone(),
                    BuildResult {
                        package_name: name.clone(),
                        status: "skipped".to_string(),
                        duration: 0.0,
                        stdout: String::new(),
                        stderr: String::new(),
                        return_code: 0,
                    },
                );
                continue;
            }

            if dry_run {
                results.lock().unwrap().insert(
                    name.clone(),
                    BuildResult {
                        package_name: name.clone(),
                        status: "would-build".to_string(),
                        duration: 0.0,
                        stdout: String::new(),
                        stderr: String::new(),
                        return_code: 0,
                    },
                );
                continue;
            }

            to_build.push(pkg);
        }

        if to_build.is_empty() || dry_run {
            continue;
        }

        // Execute this level in parallel using rayon's thread pool.
        // Each package in the level is independent of the others, so
        // they can safely run concurrently.
        pool.install(|| {
            to_build.par_iter().for_each(|pkg| {
                let result = run_package_build(pkg);

                // Update the cache based on the result.
                let pkg_hash = package_hashes
                    .get(pkg.name.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let dep_hash = deps_hashes
                    .get(pkg.name.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or("");

                if result.status == "built" {
                    build_cache.record(&pkg.name, pkg_hash, dep_hash, "success");
                } else if result.status == "failed" {
                    failed_packages.lock().unwrap().insert(pkg.name.clone());
                    build_cache.record(&pkg.name, pkg_hash, dep_hash, "failed");
                }

                results.lock().unwrap().insert(pkg.name.clone(), result);
            });
        });
    }

    results.into_inner().unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_result_defaults() {
        let result = BuildResult {
            package_name: "test/pkg".to_string(),
            status: "built".to_string(),
            duration: 1.5,
            stdout: "ok".to_string(),
            stderr: String::new(),
            return_code: 0,
        };
        assert_eq!(result.status, "built");
        assert_eq!(result.return_code, 0);
    }

    #[test]
    fn test_execute_builds_dry_run() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_exec_dry_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("BUILD"), "echo hello").unwrap();

        let packages = vec![Package {
            name: "test/pkg".to_string(),
            path: dir.clone(),
            build_commands: vec!["echo hello".to_string()],
            language: "python".to_string(),
        }];

        let mut graph = Graph::new();
        graph.add_node("test/pkg");

        let cache = BuildCache::new();
        let mut pkg_hashes = HashMap::new();
        pkg_hashes.insert("test/pkg".to_string(), "abc".to_string());
        let mut deps_hashes = HashMap::new();
        deps_hashes.insert("test/pkg".to_string(), "def".to_string());

        let results = execute_builds(
            &packages,
            &graph,
            &cache,
            &pkg_hashes,
            &deps_hashes,
            true,  // force
            true,  // dry_run
            4,     // max_jobs
            None,  // affected_set
        );

        assert_eq!(results.get("test/pkg").unwrap().status, "would-build");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_builds_skipped_by_cache() {
        let packages = vec![Package {
            name: "test/pkg".to_string(),
            path: PathBuf::from("/tmp/nonexistent"),
            build_commands: vec!["echo hello".to_string()],
            language: "python".to_string(),
        }];

        let mut graph = Graph::new();
        graph.add_node("test/pkg");

        let cache = BuildCache::new();
        cache.record("test/pkg", "abc", "def", "success");

        let mut pkg_hashes = HashMap::new();
        pkg_hashes.insert("test/pkg".to_string(), "abc".to_string());
        let mut deps_hashes = HashMap::new();
        deps_hashes.insert("test/pkg".to_string(), "def".to_string());

        let results = execute_builds(
            &packages,
            &graph,
            &cache,
            &pkg_hashes,
            &deps_hashes,
            false, // not force
            false, // not dry_run
            4,
            None,
        );

        assert_eq!(results.get("test/pkg").unwrap().status, "skipped");
    }
}
