//! # Integration Tests for head
//!
//! These tests exercise both the CLI Builder integration and the head
//! business logic. We verify line counting, byte counting, and edge
//! cases like empty input and files shorter than the requested count.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::head_tool::{head_bytes, head_lines};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("head.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load head.json");
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
        assert!(spec.is_ok(), "head.json should load successfully");
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
        match parse_argv(&["head"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn lines_flag_with_value() {
        match parse_argv(&["head", "-n", "20"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("lines").and_then(|v| v.as_i64()),
                    Some(20)
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn verbose_flag() {
        match parse_argv(&["head", "-v"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("verbose"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["head", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["head", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — head_lines
// ---------------------------------------------------------------------------

#[cfg(test)]
mod lines_logic {
    use super::*;

    #[test]
    fn first_two_lines() {
        let input = "alpha\nbeta\ngamma\ndelta\n";
        assert_eq!(head_lines(input, 2), "alpha\nbeta\n");
    }

    #[test]
    fn more_lines_than_available() {
        let input = "one\ntwo\n";
        assert_eq!(head_lines(input, 10), "one\ntwo\n");
    }

    #[test]
    fn zero_lines() {
        assert_eq!(head_lines("hello\nworld\n", 0), "");
    }

    #[test]
    fn default_ten_lines() {
        let input = (1..=15)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let result = head_lines(&input, 10);
        let count = result.matches('\n').count();
        assert_eq!(count, 10);
    }

    #[test]
    fn empty_input() {
        assert_eq!(head_lines("", 5), "");
    }

    #[test]
    fn single_line() {
        assert_eq!(head_lines("hello\n", 1), "hello\n");
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — head_bytes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod bytes_logic {
    use super::*;

    #[test]
    fn first_five_bytes() {
        assert_eq!(head_bytes(b"hello world", 5), b"hello");
    }

    #[test]
    fn more_bytes_than_available() {
        assert_eq!(head_bytes(b"hi", 100), b"hi");
    }

    #[test]
    fn zero_bytes() {
        assert_eq!(head_bytes(b"hello", 0), b"");
    }

    #[test]
    fn empty_input() {
        let empty: &[u8] = b"";
        assert_eq!(head_bytes(empty, 5), b"");
    }

    #[test]
    fn exact_length() {
        assert_eq!(head_bytes(b"abc", 3), b"abc");
    }

    #[test]
    fn single_byte() {
        assert_eq!(head_bytes(b"hello", 1), b"h");
    }
}
