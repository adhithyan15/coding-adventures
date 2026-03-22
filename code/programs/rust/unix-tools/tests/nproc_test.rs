//! # Integration Tests for nproc
//!
//! These tests verify that the `nproc` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly reports
//! processor counts with `--all` and `--ignore` flags.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::nproc_tool::{get_nproc, calculate_nproc};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("nproc.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load nproc.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads successfully
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "nproc.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Default behavior
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    #[test]
    fn no_flags_returns_parse_result() {
        match parse_argv(&["nproc"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --all flag
// ---------------------------------------------------------------------------

#[cfg(test)]
mod all_flag {
    use super::*;

    #[test]
    fn all_flag_parsed() {
        match parse_argv(&["nproc", "--all"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("all"),
                    Some(&serde_json::json!(true)),
                    "--all should set all to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --ignore flag
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ignore_flag {
    use super::*;

    #[test]
    fn ignore_flag_parsed() {
        match parse_argv(&["nproc", "--ignore", "2"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("ignore"),
                    Some(&serde_json::json!(2)),
                    "--ignore 2 should set ignore to 2"
                );
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --help and --version
// ---------------------------------------------------------------------------

#[cfg(test)]
mod builtins {
    use super::*;

    #[test]
    fn help_returns_help_result() {
        match parse_argv(&["nproc", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["nproc", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("nproc"),
                    "help text should contain 'nproc'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["nproc", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["nproc", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn nproc_returns_positive() {
        let count = get_nproc();
        assert!(count > 0, "nproc should return at least 1");
    }

    #[test]
    fn nproc_is_reasonable() {
        let count = get_nproc();
        assert!(count <= 1024, "nproc returned {} which seems unreasonable", count);
    }

    #[test]
    fn calculate_no_ignore() {
        assert_eq!(calculate_nproc(8, 0), 8);
    }

    #[test]
    fn calculate_with_ignore() {
        assert_eq!(calculate_nproc(8, 3), 5);
    }

    #[test]
    fn calculate_ignore_all_clamps_to_one() {
        assert_eq!(calculate_nproc(4, 4), 1);
    }

    #[test]
    fn calculate_ignore_more_than_available() {
        assert_eq!(calculate_nproc(4, 10), 1);
    }

    #[test]
    fn calculate_single_cpu() {
        assert_eq!(calculate_nproc(1, 0), 1);
    }

    #[test]
    fn calculate_large_system() {
        assert_eq!(calculate_nproc(256, 100), 156);
    }
}
