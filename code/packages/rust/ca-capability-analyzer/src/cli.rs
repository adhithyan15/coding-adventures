//! # Command-Line Interface
//!
//! This module implements the CLI for the capability analyzer. It provides
//! three subcommands that map to the three main use cases:
//!
//! ## Subcommands
//!
//! ### `detect` — Find all capabilities in Rust source files
//!
//! ```text
//! ca-capability-analyzer detect <path> [--exclude-tests] [--json]
//! ```
//!
//! Scans a file or directory for capability-bearing patterns and prints
//! each detected capability. Use `--json` for machine-readable output.
//!
//! ### `check` — Compare detected capabilities against a manifest
//!
//! ```text
//! ca-capability-analyzer check <path> --manifest <manifest.json> [--exclude-tests]
//! ```
//!
//! Runs detection and then compares results against a
//! `required_capabilities.json` manifest. Exits with code 0 if all
//! capabilities are declared, code 1 if any are undeclared.
//!
//! ### `banned` — Detect banned constructs (unsafe, extern, etc.)
//!
//! ```text
//! ca-capability-analyzer banned <path> [--exclude-tests] [--json]
//! ```
//!
//! Specifically looks for banned constructs: `unsafe` blocks,
//! `extern "C"` declarations, `mem::transmute`, and `include_bytes!`/
//! `include_str!` macros. These are a subset of all detectable
//! capabilities focused on FFI and safety-bypassing patterns.
//!
//! ## Design Note
//!
//! We use `std::env::args()` directly rather than an external CLI
//! parsing library (like clap). This keeps dependencies minimal — the
//! analyzer should itself have a small capability surface area.

use crate::analyzer::{self, DetectedCapability};
use crate::manifest;

/// Run the CLI with the given arguments.
///
/// Returns an exit code: 0 for success, 1 for failure, 2 for usage errors.
///
/// # Arguments
///
/// * `args` — Command-line arguments (including the program name at index 0)
pub fn run(args: &[String]) -> i32 {
    if args.len() < 2 {
        print_usage();
        return 2;
    }

    match args[1].as_str() {
        "detect" => cmd_detect(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "banned" => cmd_banned(&args[2..]),
        "help" | "--help" | "-h" => {
            print_usage();
            0
        }
        other => {
            eprintln!("Unknown subcommand: {}", other);
            print_usage();
            2
        }
    }
}

/// Print usage information.
fn print_usage() {
    eprintln!(
        "Usage: ca-capability-analyzer <subcommand> [options]

Subcommands:
  detect <path> [--exclude-tests] [--json]
      Detect all OS capabilities in Rust source files.

  check <path> --manifest <manifest.json> [--exclude-tests]
      Compare detected capabilities against a manifest.

  banned <path> [--exclude-tests] [--json]
      Detect banned constructs (unsafe, extern, transmute, include_*).

  help
      Show this help message."
    );
}

// ── Argument Parsing Helpers ─────────────────────────────────────────
//
// Since we're using std::env::args() directly, we need small helpers
// to extract flags and named arguments from the argument list.

/// Check if a flag (e.g., "--json") is present in the arguments.
fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

/// Get the value of a named argument (e.g., "--manifest path.json").
///
/// Returns the argument following the named flag, or None if not found.
fn get_named_arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    for (i, arg) in args.iter().enumerate() {
        if arg == name && i + 1 < args.len() {
            return Some(&args[i + 1]);
        }
    }
    None
}

/// Get the first positional argument (not a flag or named arg value).
///
/// Skips arguments that start with "--" and their values.
fn get_positional_arg(args: &[String]) -> Option<&str> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--manifest" {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--") {
            continue;
        }
        return Some(arg);
    }
    None
}

// ── Subcommand: detect ───────────────────────────────────────────────

/// Run the `detect` subcommand.
///
/// Scans the given path for capabilities and prints results.
fn cmd_detect(args: &[String]) -> i32 {
    let path = match get_positional_arg(args) {
        Some(p) => p,
        None => {
            eprintln!("Error: detect requires a path argument.");
            return 2;
        }
    };

    let exclude_tests = has_flag(args, "--exclude-tests");
    let json_output = has_flag(args, "--json");

    let capabilities = detect_capabilities(path, exclude_tests);
    let capabilities = match capabilities {
        Ok(caps) => caps,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    if json_output {
        print_json(&capabilities);
    } else {
        print_human(&capabilities);
    }

    0
}

// ── Subcommand: check ────────────────────────────────────────────────

/// Run the `check` subcommand.
///
/// Detects capabilities and compares against a manifest.
/// Returns 0 if all capabilities are declared, 1 if any are undeclared.
fn cmd_check(args: &[String]) -> i32 {
    let path = match get_positional_arg(args) {
        Some(p) => p,
        None => {
            eprintln!("Error: check requires a path argument.");
            return 2;
        }
    };

    let manifest_path = match get_named_arg(args, "--manifest") {
        Some(p) => p,
        None => {
            eprintln!("Error: check requires --manifest <path>.");
            return 2;
        }
    };

    let exclude_tests = has_flag(args, "--exclude-tests");

    let capabilities = match detect_capabilities(path, exclude_tests) {
        Ok(caps) => caps,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let m = match manifest::load_manifest(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading manifest: {}", e);
            return 1;
        }
    };

    let result = manifest::compare_capabilities(&capabilities, &m);
    println!("{}", result.summary());

    if result.passed {
        0
    } else {
        1
    }
}

// ── Subcommand: banned ───────────────────────────────────────────────

/// Run the `banned` subcommand.
///
/// Specifically looks for banned constructs: unsafe blocks, extern blocks,
/// mem::transmute, and include_bytes!/include_str! macros.
fn cmd_banned(args: &[String]) -> i32 {
    let path = match get_positional_arg(args) {
        Some(p) => p,
        None => {
            eprintln!("Error: banned requires a path argument.");
            return 2;
        }
    };

    let exclude_tests = has_flag(args, "--exclude-tests");
    let json_output = has_flag(args, "--json");

    let capabilities = match detect_capabilities(path, exclude_tests) {
        Ok(caps) => caps,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    // Filter to only banned constructs (ffi category + include_* macros)
    let banned: Vec<_> = capabilities
        .into_iter()
        .filter(|c| {
            c.category == "ffi"
                || c.evidence.starts_with("include_bytes!")
                || c.evidence.starts_with("include_str!")
        })
        .collect();

    if json_output {
        print_json(&banned);
    } else {
        if banned.is_empty() {
            println!("No banned constructs detected.");
        } else {
            println!(
                "Found {} banned construct(s):\n",
                banned.len()
            );
            print_human(&banned);
        }
    }

    if banned.is_empty() {
        0
    } else {
        1
    }
}

// ── Detection Helper ─────────────────────────────────────────────────

/// Detect capabilities in a file or directory.
///
/// Determines whether the path is a file or directory and calls the
/// appropriate analyzer function.
fn detect_capabilities(
    path: &str,
    exclude_tests: bool,
) -> Result<Vec<DetectedCapability>, String> {
    let p = std::path::Path::new(path);

    if p.is_file() {
        analyzer::analyze_file(path)
    } else if p.is_dir() {
        analyzer::analyze_directory(path, exclude_tests)
    } else {
        Err(format!("{} is not a file or directory", path))
    }
}

// ── Output Formatting ────────────────────────────────────────────────

/// Print capabilities as human-readable text.
fn print_human(capabilities: &[DetectedCapability]) {
    if capabilities.is_empty() {
        println!("No capabilities detected.");
        return;
    }

    println!("Detected {} capability(ies):\n", capabilities.len());
    for cap in capabilities {
        println!(
            "  {}:{}: {} ({})",
            cap.file, cap.line, cap, cap.evidence
        );
    }
}

/// Print capabilities as JSON.
fn print_json(capabilities: &[DetectedCapability]) {
    let json = serde_json::to_string_pretty(capabilities).unwrap_or_else(|_| "[]".to_string());
    println!("{}", json);
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Argument parsing tests ───────────────────────────────────────

    #[test]
    fn test_has_flag() {
        let args = vec!["--json".to_string(), "path".to_string()];
        assert!(has_flag(&args, "--json"));
        assert!(!has_flag(&args, "--exclude-tests"));
    }

    #[test]
    fn test_get_named_arg() {
        let args = vec![
            "path".to_string(),
            "--manifest".to_string(),
            "caps.json".to_string(),
        ];
        assert_eq!(get_named_arg(&args, "--manifest"), Some("caps.json"));
        assert_eq!(get_named_arg(&args, "--other"), None);
    }

    #[test]
    fn test_get_named_arg_at_end() {
        // If --manifest is the last argument with no value, return None
        let args = vec!["path".to_string(), "--manifest".to_string()];
        assert_eq!(get_named_arg(&args, "--manifest"), None);
    }

    #[test]
    fn test_get_positional_arg() {
        let args = vec![
            "--json".to_string(),
            "src/".to_string(),
        ];
        assert_eq!(get_positional_arg(&args), Some("src/"));
    }

    #[test]
    fn test_get_positional_arg_with_manifest() {
        let args = vec![
            "src/".to_string(),
            "--manifest".to_string(),
            "caps.json".to_string(),
        ];
        assert_eq!(get_positional_arg(&args), Some("src/"));
    }

    #[test]
    fn test_get_positional_arg_none() {
        let args = vec!["--json".to_string(), "--exclude-tests".to_string()];
        assert_eq!(get_positional_arg(&args), None);
    }

    // ── CLI run tests ────────────────────────────────────────────────

    #[test]
    fn test_run_no_args() {
        let args = vec!["ca-capability-analyzer".to_string()];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_help() {
        let args = vec!["ca-capability-analyzer".to_string(), "help".to_string()];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn test_run_unknown_subcommand() {
        let args = vec!["ca-capability-analyzer".to_string(), "unknown".to_string()];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_detect_no_path() {
        let args = vec!["ca-capability-analyzer".to_string(), "detect".to_string()];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_check_no_path() {
        let args = vec!["ca-capability-analyzer".to_string(), "check".to_string()];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_check_no_manifest() {
        let args = vec![
            "ca-capability-analyzer".to_string(),
            "check".to_string(),
            "src/".to_string(),
        ];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_banned_no_path() {
        let args = vec!["ca-capability-analyzer".to_string(), "banned".to_string()];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn test_run_detect_nonexistent_path() {
        let args = vec![
            "ca-capability-analyzer".to_string(),
            "detect".to_string(),
            "/nonexistent/path/xyz".to_string(),
        ];
        assert_eq!(run(&args), 1);
    }

    #[test]
    fn test_run_banned_nonexistent_path() {
        let args = vec![
            "ca-capability-analyzer".to_string(),
            "banned".to_string(),
            "/nonexistent/path/xyz".to_string(),
        ];
        assert_eq!(run(&args), 1);
    }
}
