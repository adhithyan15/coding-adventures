//! # Integration Tests for yes
//!
//! These tests exercise both the CLI Builder integration (spec parsing)
//! and the `yes` business logic (output generation with default and
//! custom strings).
//!
//! ## Test Strategy
//!
//! 1. **Spec-level tests**: Verify the JSON spec integrates correctly
//!    with CLI Builder.
//! 2. **Logic-level tests**: Verify `yes_output` and `join_args`
//!    produce correct output.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::yes_tool::{yes_output, join_args};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("yes.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load yes.json");
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
        assert!(spec.is_ok(), "yes.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Default behavior (no arguments)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    #[test]
    fn no_args_returns_parse_result() {
        match parse_argv(&["yes"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn with_argument_returns_parse_result() {
        match parse_argv(&["yes", "hello"]) {
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
        match parse_argv(&["yes", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["yes", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("yes"),
                    "help text should contain 'yes'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["yes", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["yes", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — yes_output
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn default_output_is_y() {
        let result = yes_output("", 3);
        assert_eq!(result, "y\ny\ny\n");
    }

    #[test]
    fn custom_string() {
        let result = yes_output("hello", 2);
        assert_eq!(result, "hello\nhello\n");
    }

    #[test]
    fn zero_lines_produces_empty() {
        let result = yes_output("y", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn single_line() {
        let result = yes_output("test", 1);
        assert_eq!(result, "test\n");
    }

    #[test]
    fn multi_word_string() {
        let result = yes_output("hello world", 2);
        assert_eq!(result, "hello world\nhello world\n");
    }

    #[test]
    fn join_multiple_args() {
        let args: Vec<String> = vec!["hello".into(), "world".into()];
        assert_eq!(join_args(&args), "hello world");
    }

    #[test]
    fn join_empty_args() {
        let args: Vec<String> = vec![];
        assert_eq!(join_args(&args), "");
    }

    #[test]
    fn large_output_correct_line_count() {
        let result = yes_output("y", 50);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 50);
    }

    #[test]
    fn all_lines_identical() {
        let result = yes_output("abc", 10);
        for line in result.lines() {
            assert_eq!(line, "abc");
        }
    }
}
