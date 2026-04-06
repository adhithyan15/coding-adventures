//! # Integration Tests for whoami
//!
//! These tests verify that the `whoami` JSON spec integrates correctly
//! with CLI Builder, and that the business logic returns a valid
//! username from the environment.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use std::path::PathBuf;
use unix_tools::whoami_tool::get_username;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("whoami.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load whoami.json");
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
        assert!(spec.is_ok(), "whoami.json should load successfully");
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
        match parse_argv(&["whoami"]) {
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
        match parse_argv(&["whoami", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["whoami", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("whoami"),
                    "help text should contain 'whoami'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["whoami", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["whoami", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — get_username
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    // These tests rely on $USER/$LOGNAME env vars which are Unix-specific.
    // Windows uses $USERNAME instead, and neither $USER nor $LOGNAME is
    // typically set on Windows CI.

    #[cfg(unix)]
    #[test]
    fn returns_a_username() {
        let result = get_username();
        assert!(result.is_ok(), "get_username should succeed");
        let name = result.unwrap();
        assert!(!name.is_empty(), "username should not be empty");
    }

    #[cfg(unix)]
    #[test]
    fn username_has_no_newlines() {
        let name = get_username().unwrap();
        if name.contains('\n') {
            panic!("username should not contain newlines");
        }
    }

    #[cfg(unix)]
    #[test]
    fn username_is_consistent() {
        let first = get_username().unwrap();
        let second = get_username().unwrap();
        if first != second {
            panic!("username should be consistent across calls");
        }
    }

    #[cfg(unix)]
    #[test]
    fn username_matches_env() {
        // The username should match $USER or $LOGNAME.
        let name = get_username().unwrap();
        let user_var = std::env::var("USER").ok();
        let logname_var = std::env::var("LOGNAME").ok();
        if user_var.as_deref() != Some(name.as_str())
            && logname_var.as_deref() != Some(name.as_str())
        {
            panic!("username should match $USER or $LOGNAME");
        }
    }
}
