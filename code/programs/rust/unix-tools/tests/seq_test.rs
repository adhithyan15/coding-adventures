//! # Integration Tests for seq
//!
//! These tests exercise both the CLI Builder integration and the seq
//! business logic. We test simple sequences, custom separators,
//! counting down, equal-width padding, and floating-point sequences.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::seq_tool::generate_sequence;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("seq.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load seq.json");
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
        assert!(spec.is_ok(), "seq.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn with_single_number() {
        match parse_argv(&["seq", "5"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn equal_width_flag() {
        match parse_argv(&["seq", "-w", "1", "10"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("equal_width"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn separator_flag() {
        match parse_argv(&["seq", "-s", ", ", "1", "3"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("separator").and_then(|v| v.as_str()),
                    Some(", ")
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["seq", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["seq", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — generate_sequence
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn simple_1_to_5() {
        assert_eq!(
            generate_sequence(1.0, 1.0, 5.0, "\n", false),
            "1\n2\n3\n4\n5\n"
        );
    }

    #[test]
    fn custom_separator() {
        assert_eq!(
            generate_sequence(1.0, 1.0, 3.0, ", ", false),
            "1, 2, 3\n"
        );
    }

    #[test]
    fn step_of_two() {
        assert_eq!(
            generate_sequence(1.0, 2.0, 7.0, "\n", false),
            "1\n3\n5\n7\n"
        );
    }

    #[test]
    fn counting_down() {
        assert_eq!(
            generate_sequence(5.0, -1.0, 1.0, "\n", false),
            "5\n4\n3\n2\n1\n"
        );
    }

    #[test]
    fn single_value() {
        assert_eq!(generate_sequence(5.0, 1.0, 5.0, "\n", false), "5\n");
    }

    #[test]
    fn equal_width_padding() {
        assert_eq!(
            generate_sequence(8.0, 1.0, 11.0, "\n", true),
            "08\n09\n10\n11\n"
        );
    }

    #[test]
    fn impossible_sequence() {
        assert_eq!(generate_sequence(5.0, 1.0, 1.0, "\n", false), "");
    }

    #[test]
    fn zero_increment() {
        assert_eq!(generate_sequence(1.0, 0.0, 5.0, "\n", false), "");
    }

    #[test]
    fn fractional_step() {
        assert_eq!(
            generate_sequence(0.0, 0.5, 1.5, "\n", false),
            "0.0\n0.5\n1.0\n1.5\n"
        );
    }
}
