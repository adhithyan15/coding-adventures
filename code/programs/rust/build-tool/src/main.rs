// Build Tool — Incremental, Parallel Monorepo Build System (Rust)
//
// This is a complete Rust port of the Go build tool for the coding-adventures
// monorepo. It discovers packages via recursive BUILD file walking, resolves dependencies,
// hashes source files, and only rebuilds packages whose source (or dependency
// source) has changed. Independent packages are built in parallel using
// Rayon's work-stealing thread pool.
//
// # The build flow
//
//  1. Find the repo root (walk up looking for .git)
//  2. Discover packages (walk BUILD files under code/)
//  3. Filter by language if requested
//  4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, Cargo.toml)
//  5. Hash all packages and their dependencies
//  6. Load cache, determine what needs building
//  7. If --dry-run, report what would build and exit
//  8. Execute builds in parallel by dependency level
//  9. Update and save cache
//  10. Print report
//  11. Exit with code 1 if any builds failed
//
// # Why Rust?
//
// The Rust implementation complements the Go primary tool:
//   - Zero-cost abstractions and compile-time safety guarantees.
//   - Rayon provides work-stealing parallelism (vs Go's goroutine model).
//   - No garbage collector — predictable, low-latency performance.
//   - Strong type system catches dependency graph errors at compile time.
//   - Demonstrates the same algorithms in a different systems language.

mod cache;
mod ci_workflow;
mod discovery;
mod executor;
mod gitdiff;
pub mod glob_match;
mod graph;
mod hasher;
pub mod plan;
mod reporter;
mod resolver;
mod validator;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

/// Incremental, parallel monorepo build system.
///
/// Discovers packages via BUILD files, resolves dependencies, hashes source
/// files for change detection, and rebuilds only what changed. Independent
/// packages are built in parallel.
#[derive(Parser, Debug)]
#[command(name = "build-tool", version = "0.1.0")]
struct Args {
    /// Repo root directory (auto-detect from .git if omitted).
    #[arg(long)]
    root: Option<String>,

    /// Git ref to diff against for change detection.
    #[arg(long, default_value = "origin/main")]
    diff_base: String,

    /// Rebuild everything regardless of cache.
    #[arg(long)]
    force: bool,

    /// Show what would build without executing.
    #[arg(long)]
    dry_run: bool,

    /// Max parallel jobs (default: CPU count).
    #[arg(long)]
    jobs: Option<usize>,

    /// Filter to language: python, ruby, go, typescript, rust, elixir, lua,
    /// perl, swift, haskell, wasm, csharp, fsharp, dotnet, or all.
    #[arg(long, default_value = "all")]
    language: String,

    /// Path to cache file.
    #[arg(long, default_value = ".build-cache.json")]
    cache_file: String,

    /// Emit a build plan (JSON) describing what would be built.
    ///
    /// When enabled, the tool writes a JSON file containing every package
    /// that would be built, along with its commands and the reason for
    /// rebuilding. This is useful for CI auditing and debugging.
    #[arg(long)]
    emit_plan: bool,

    /// Path to the build plan file (used with --emit-plan).
    ///
    /// Defaults to "build-plan.json" in the repo root. The plan is
    /// written as pretty-printed JSON for human readability.
    #[arg(long, default_value = "build-plan.json")]
    plan_file: String,

    /// Validate BUILD/CI metadata contracts before continuing.
    #[arg(long)]
    validate_build_files: bool,
}

// ---------------------------------------------------------------------------
// Repo root detection
// ---------------------------------------------------------------------------

/// Walks up from the given directory (or cwd) looking for a .git directory.
/// This is how we auto-detect the repo root without requiring the user to
/// pass --root every time.
fn find_repo_root(start: Option<&str>) -> Option<PathBuf> {
    let start_path = match start {
        Some(s) => PathBuf::from(s),
        None => std::env::current_dir().ok()?,
    };

    let mut current = start_path.canonicalize().ok()?;

    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => {
                if parent == current {
                    // Reached filesystem root without finding .git.
                    return None;
                }
                current = parent.to_path_buf();
            }
            None => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------

/// Produces an ISO 8601 timestamp string for the current time.
///
/// We avoid pulling in a full datetime library (chrono) for this single use.
/// Instead, we use `std::time::SystemTime` and format it manually. The
/// output looks like "2026-03-22T10:30:00Z" — sufficient for build plan
/// audit logs. The precision is seconds (no sub-second component).
fn chrono_now_iso8601() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert Unix timestamp to broken-down UTC components.
    // This is the same algorithm used by gmtime() in C.
    let secs_per_day: u64 = 86400;
    let days = now / secs_per_day;
    let day_secs = now % secs_per_day;

    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    // Calculate year, month, day from days since epoch (1970-01-01).
    let mut y = 1970u64;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let leap = is_leap_year(y);
    let month_days: [u64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0u64;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining_days < md {
            m = i as u64 + 1;
            break;
        }
        remaining_days -= md;
    }

    let d = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

/// Helper for leap year calculation.
fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    process::exit(run());
}

/// Contains the actual logic, separated from main() so we can return an
/// exit code cleanly. This mirrors the Go implementation's run() function.
fn run() -> i32 {
    let args = Args::parse();

    // Step 1: Find the repo root.
    let repo_root = match &args.root {
        Some(root) => {
            let path = PathBuf::from(root);
            match path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            }
        }
        None => match find_repo_root(None) {
            Some(r) => r,
            None => {
                eprintln!("Error: Could not find repo root (.git directory).");
                eprintln!("Use --root to specify the repo root.");
                return 1;
            }
        },
    };

    // The build starts from the code/ directory inside the repo root.
    let code_root = repo_root.join("code");
    if !code_root.is_dir() {
        eprintln!(
            "Error: {} does not exist or is not a directory.",
            code_root.display()
        );
        return 1;
    }

    // Step 2: Discover packages.
    let mut packages = discovery::discover_packages(&code_root);
    if packages.is_empty() {
        eprintln!("No packages found.");
        return 0;
    }

    // Step 3: Filter by language if requested.
    if args.language != "all" {
        packages.retain(|pkg| pkg.language == args.language);
        if packages.is_empty() {
            eprintln!("No {} packages found.", args.language);
            return 0;
        }
    }

    if args.validate_build_files {
        if let Some(validation_error) = validator::validate_build_contracts(&repo_root, &packages) {
            eprintln!("BUILD/CI validation failed:");
            eprintln!("  - {}", validation_error);
            eprintln!(
                "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct."
            );
            return 1;
        }
    }

    println!("Discovered {} packages", packages.len());

    // Step 4: Resolve dependencies.
    let graph = resolver::resolve_dependencies(&packages);

    // Step 5: Git-diff change detection (default mode).
    // Git is the source of truth — no cache file needed for primary workflow.
    let mut force = args.force;
    let affected_set = if !force {
        let changed_files = gitdiff::get_changed_files(&repo_root, &args.diff_base);
        if !changed_files.is_empty() {
            if changed_files
                .iter()
                .any(|path| path == ci_workflow::CI_WORKFLOW_PATH)
            {
                let ci_change =
                    ci_workflow::analyze_ci_workflow_changes(&repo_root, &args.diff_base);
                if ci_change.requires_full_rebuild {
                    println!("Git diff: ci.yml changed in shared ways — rebuilding everything");
                    force = true;
                    None
                } else {
                    let ci_toolchains = ci_workflow::sorted_toolchains(&ci_change.toolchains);
                    if !ci_toolchains.is_empty() {
                        println!(
                            "Git diff: ci.yml changed only toolchain-scoped setup for {}",
                            ci_toolchains.join(", ")
                        );
                    }

                    let changed_pkgs =
                        gitdiff::map_files_to_packages(&changed_files, &packages, &repo_root);
                    if !changed_pkgs.is_empty() {
                        let affected = graph.affected_nodes(&changed_pkgs);
                        println!(
                            "Git diff: {} packages changed, {} affected (including dependents)",
                            changed_pkgs.len(),
                            affected.len()
                        );
                        Some(affected)
                    } else {
                        println!("Git diff: no package files changed — nothing to build");
                        Some(std::collections::HashSet::new()) // empty = build nothing
                    }
                }
            } else {
                let changed_pkgs =
                    gitdiff::map_files_to_packages(&changed_files, &packages, &repo_root);
                if !changed_pkgs.is_empty() {
                    let affected = graph.affected_nodes(&changed_pkgs);
                    println!(
                        "Git diff: {} packages changed, {} affected (including dependents)",
                        changed_pkgs.len(),
                        affected.len()
                    );
                    Some(affected)
                } else {
                    println!("Git diff: no package files changed — nothing to build");
                    Some(std::collections::HashSet::new()) // empty = build nothing
                }
            }
        } else {
            println!("Git diff unavailable — falling back to hash-based cache");
            None
        }
    } else {
        None
    };

    // Step 6: Hash all packages (needed for cache fallback).
    let mut package_hashes: HashMap<String, String> = HashMap::new();
    let mut deps_hashes: HashMap<String, String> = HashMap::new();

    for pkg in &packages {
        package_hashes.insert(pkg.name.clone(), hasher::hash_package(pkg));
        deps_hashes.insert(
            pkg.name.clone(),
            hasher::hash_deps(&pkg.name, &graph, &package_hashes),
        );
    }

    // Step 7: Load cache (fallback if git diff didn't work).
    let cache_path = if Path::new(&args.cache_file).is_absolute() {
        PathBuf::from(&args.cache_file)
    } else {
        repo_root.join(&args.cache_file)
    };

    let build_cache = cache::BuildCache::new();
    build_cache.load(&cache_path);

    // Step 7b: Emit build plan if requested.
    //
    // The plan is emitted BEFORE executing builds so that:
    //   - In CI, the plan is available even if the build crashes.
    //   - With --dry-run + --emit-plan, you get a plan without building.
    if args.emit_plan {
        let plan_entries: Vec<plan::PackageEntry> = packages
            .iter()
            .filter(|pkg| {
                // Only include packages that would actually be built.
                if force {
                    return true;
                }
                if let Some(ref affected) = affected_set {
                    return affected.contains(&pkg.name);
                }
                // Fallback: check cache.
                let pkg_hash = package_hashes.get(&pkg.name).cloned().unwrap_or_default();
                let dep_hash = deps_hashes.get(&pkg.name).cloned().unwrap_or_default();
                build_cache.needs_build(&pkg.name, &pkg_hash, &dep_hash)
            })
            .map(|pkg| {
                let reason = if force {
                    "forced".to_string()
                } else if let Some(ref affected) = affected_set {
                    // Determine if this package changed directly or via dependency.
                    let changed_files = gitdiff::get_changed_files(&repo_root, &args.diff_base);
                    let directly_changed =
                        gitdiff::map_files_to_packages(&changed_files, &packages, &repo_root);
                    if directly_changed.contains(&pkg.name) {
                        "changed".to_string()
                    } else if affected.contains(&pkg.name) {
                        "dependency_changed".to_string()
                    } else {
                        "cache_miss".to_string()
                    }
                } else {
                    "cache_miss".to_string()
                };

                plan::PackageEntry {
                    name: pkg.name.clone(),
                    language: pkg.language.clone(),
                    commands: pkg.build_commands.clone(),
                    reason,
                }
            })
            .collect();

        let build_plan = plan::BuildPlan {
            schema_version: plan::CURRENT_SCHEMA_VERSION,
            created_at: chrono_now_iso8601(),
            diff_base: args.diff_base.clone(),
            packages: plan_entries,
        };

        let plan_path = if Path::new(&args.plan_file).is_absolute() {
            args.plan_file.clone()
        } else {
            repo_root
                .join(&args.plan_file)
                .to_string_lossy()
                .to_string()
        };

        match plan::write_plan(&build_plan, &plan_path) {
            Ok(()) => println!("Build plan written to {}", plan_path),
            Err(e) => eprintln!("Warning: could not write build plan: {}", e),
        }
    }

    // Steps 8-9: Execute builds.
    let max_jobs = args.jobs.unwrap_or_else(num_cpus::get);

    let results = executor::execute_builds(
        &packages,
        &graph,
        &build_cache,
        &package_hashes,
        &deps_hashes,
        force,
        args.dry_run,
        max_jobs,
        affected_set.as_ref(),
    );

    // Step 10: Save cache (secondary record, not primary mechanism).
    if !args.dry_run {
        if let Err(e) = build_cache.save(&cache_path) {
            eprintln!("Warning: could not save cache: {}", e);
        }
    }

    // Step 10: Print report.
    reporter::print_report(&results, None);

    // Step 11: Exit with code 1 if any builds failed.
    for r in results.values() {
        if r.status == "failed" {
            return 1;
        }
    }

    0
}
