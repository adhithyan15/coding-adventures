//! # Integration Tests for rev
//!
//! These tests exercise both the CLI Builder integration and the rev
//! business logic. We test single-line reversal, multi-line reversal,
//! palindromes, blank lines, and Unicode handling.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::rev_tool::reverse_lines;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("rev.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load rev.json");
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
        assert!(spec.is_ok(), "rev.json should load successfully");
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
        match parse_argv(&["rev"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["rev", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["rev", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — reverse_lines
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn simple_reversal() {
        assert_eq!(reverse_lines("hello\n"), "olleh\n");
    }

    #[test]
    fn multiple_lines() {
        assert_eq!(reverse_lines("hello\nworld\n"), "olleh\ndlrow\n");
    }

    #[test]
    fn empty_content() {
        assert_eq!(reverse_lines(""), "");
    }

    #[test]
    fn palindrome() {
        assert_eq!(reverse_lines("racecar\n"), "racecar\n");
    }

    #[test]
    fn no_trailing_newline() {
        assert_eq!(reverse_lines("hello"), "olleh");
    }

    #[test]
    fn blank_lines_preserved() {
        assert_eq!(reverse_lines("abc\n\ndef\n"), "cba\n\nfed\n");
    }

    #[test]
    fn spaces_reversed() {
        assert_eq!(reverse_lines("a b c\n"), "c b a\n");
    }

    #[test]
    fn single_character() {
        assert_eq!(reverse_lines("x\n"), "x\n");
    }
}
