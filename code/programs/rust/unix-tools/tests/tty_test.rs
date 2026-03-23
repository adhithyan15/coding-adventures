//! # Integration Tests for tty
//!
//! These tests verify that the `tty` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly identifies
//! terminal status and formats output.
//!
//! ## Note on Test Environment
//!
//! When running under `cargo test`, stdin is NOT connected to a
//! terminal. So `check_tty()` will report "not a tty" with exit
//! code 1. We test both this expected behavior and the formatting
//! functions with synthetic data.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::tty_tool::{check_tty, format_tty_output, TtyResult};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("tty.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load tty.json");
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
        assert!(spec.is_ok(), "tty.json should load successfully");
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
        match parse_argv(&["tty"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: -s flag (silent mode)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod silent_flag {
    use super::*;

    #[test]
    fn short_flag_parsed() {
        match parse_argv(&["tty", "-s"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("silent"),
                    Some(&serde_json::json!(true)),
                    "-s should set silent to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn long_flag_parsed() {
        match parse_argv(&["tty", "--silent"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("silent"),
                    Some(&serde_json::json!(true)),
                    "--silent should set silent to true"
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
        match parse_argv(&["tty", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["tty", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("tty"),
                    "help text should contain 'tty'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["tty", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["tty", "--version"]) {
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

    #[cfg(unix)]
    #[test]
    fn check_tty_in_test_env() {
        // Under cargo test, stdin is not a terminal.
        let result = check_tty();
        assert_eq!(result.name, "not a tty");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn format_normal_with_tty() {
        let result = TtyResult {
            name: "/dev/pts/0".to_string(),
            exit_code: 0,
        };
        assert_eq!(format_tty_output(&result, false), "/dev/pts/0\n");
    }

    #[test]
    fn format_normal_not_tty() {
        let result = TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        };
        assert_eq!(format_tty_output(&result, false), "not a tty\n");
    }

    #[test]
    fn format_silent_produces_empty() {
        let result = TtyResult {
            name: "/dev/pts/0".to_string(),
            exit_code: 0,
        };
        assert_eq!(format_tty_output(&result, true), "");
    }

    #[test]
    fn format_silent_not_tty_produces_empty() {
        let result = TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        };
        assert_eq!(format_tty_output(&result, true), "");
    }

    #[cfg(unix)]
    #[test]
    fn exit_code_matches_tty_status() {
        let result = check_tty();
        if result.name == "not a tty" {
            assert_eq!(result.exit_code, 1);
        } else {
            assert_eq!(result.exit_code, 0);
        }
    }
}
