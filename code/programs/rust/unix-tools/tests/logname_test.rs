//! # Integration Tests for logname
//!
//! These tests verify that the `logname` JSON spec integrates correctly
//! with CLI Builder, and that the business logic returns a valid
//! login name from the environment.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::logname_tool::get_logname;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("logname.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load logname.json");
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
        assert!(spec.is_ok(), "logname.json should load successfully");
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
        match parse_argv(&["logname"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
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
        match parse_argv(&["logname", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["logname", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("logname"),
                    "help text should contain 'logname'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["logname", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["logname", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — get_logname
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn returns_a_login_name() {
        let result = get_logname();
        assert!(result.is_ok(), "get_logname should succeed");
        let name = result.unwrap();
        assert!(!name.is_empty(), "login name should not be empty");
    }

    #[test]
    fn login_name_has_no_newlines() {
        let name = get_logname().unwrap();
        assert!(
            !name.contains('\n'),
            "login name should not contain newlines, got: '{}'",
            name
        );
    }

    #[test]
    fn login_name_is_consistent() {
        let first = get_logname().unwrap();
        let second = get_logname().unwrap();
        assert_eq!(first, second, "login name should be consistent across calls");
    }

    #[test]
    fn login_name_matches_env() {
        let name = get_logname().unwrap();
        let logname_var = std::env::var("LOGNAME").ok();
        let user_var = std::env::var("USER").ok();
        assert!(
            logname_var.as_deref() == Some(&name[..]) || user_var.as_deref() == Some(&name[..]),
            "login name '{}' should match $LOGNAME or $USER",
            name
        );
    }
}
