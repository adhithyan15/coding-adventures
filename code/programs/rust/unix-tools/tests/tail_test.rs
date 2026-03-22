//! # Integration Tests for tail
//!
//! These tests exercise both the CLI Builder integration and the tail
//! business logic. We test both "last N lines" mode and "+N" (from
//! line N) mode, plus edge cases.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::tail_tool::{tail_from_line, tail_lines};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("tail.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load tail.json");
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
        assert!(spec.is_ok(), "tail.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn no_flags_returns_parse() {
        match parse_argv(&["tail"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn follow_flag() {
        match parse_argv(&["tail", "-f"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("follow"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["tail", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["tail", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — tail_lines
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tail_lines_logic {
    use super::*;

    #[test]
    fn last_two_lines() {
        let input = "alpha\nbeta\ngamma\ndelta\n";
        assert_eq!(tail_lines(input, 2), "gamma\ndelta\n");
    }

    #[test]
    fn more_lines_than_available() {
        let input = "one\ntwo\n";
        assert_eq!(tail_lines(input, 10), "one\ntwo\n");
    }

    #[test]
    fn zero_lines() {
        assert_eq!(tail_lines("hello\nworld\n", 0), "");
    }

    #[test]
    fn single_line() {
        assert_eq!(tail_lines("hello\n", 1), "hello\n");
    }

    #[test]
    fn empty_input() {
        assert_eq!(tail_lines("", 5), "");
    }

    #[test]
    fn last_line_of_many() {
        let input = "a\nb\nc\nd\ne\n";
        assert_eq!(tail_lines(input, 1), "e\n");
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — tail_from_line
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tail_from_line_logic {
    use super::*;

    #[test]
    fn from_line_three() {
        let input = "alpha\nbeta\ngamma\ndelta\nepsilon\n";
        assert_eq!(tail_from_line(input, 3), "gamma\ndelta\nepsilon\n");
    }

    #[test]
    fn from_line_one_is_everything() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 1), "alpha\nbeta\n");
    }

    #[test]
    fn from_line_beyond_end() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 10), "");
    }

    #[test]
    fn from_line_zero_is_everything() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 0), "alpha\nbeta\n");
    }

    #[test]
    fn from_last_line() {
        let input = "a\nb\nc\n";
        assert_eq!(tail_from_line(input, 3), "c\n");
    }

    #[test]
    fn from_line_two() {
        let input = "first\nsecond\nthird\n";
        assert_eq!(tail_from_line(input, 2), "second\nthird\n");
    }
}
