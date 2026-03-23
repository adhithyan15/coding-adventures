//! # Integration Tests for cut
//!
//! These tests verify that the `cut` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly extracts
//! bytes, characters, and fields from lines.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::cut_tool::{cut_bytes, cut_characters, cut_fields, parse_range_list, CutOptions, Range};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("cut.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load cut.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "cut.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn help() {
        match parse_argv(&["cut", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("cut")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["cut", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Range parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod range_parsing {
    use super::*;

    #[test]
    fn single_field() {
        let ranges = parse_range_list("3").unwrap();
        assert_eq!(ranges, vec![Range::Single(3)]);
    }

    #[test]
    fn closed_range() {
        let ranges = parse_range_list("2-5").unwrap();
        assert_eq!(ranges, vec![Range::Closed(2, 5)]);
    }

    #[test]
    fn from_range() {
        let ranges = parse_range_list("3-").unwrap();
        assert_eq!(ranges, vec![Range::From(3)]);
    }

    #[test]
    fn to_range() {
        let ranges = parse_range_list("-4").unwrap();
        assert_eq!(ranges, vec![Range::To(4)]);
    }

    #[test]
    fn complex_range() {
        let ranges = parse_range_list("1,3-5,7-").unwrap();
        assert_eq!(ranges.len(), 3);
    }

    #[test]
    fn zero_is_error() {
        assert!(parse_range_list("0").is_err());
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn cut_single_character() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1)],
            ..Default::default()
        };
        assert_eq!(cut_characters("hello", &opts), "h");
    }

    #[test]
    fn cut_character_range() {
        let opts = CutOptions {
            ranges: vec![Range::Closed(1, 5)],
            ..Default::default()
        };
        assert_eq!(cut_characters("hello world", &opts), "hello");
    }

    #[test]
    fn cut_bytes_range() {
        let opts = CutOptions {
            ranges: vec![Range::Closed(1, 5)],
            ..Default::default()
        };
        assert_eq!(cut_bytes("hello world", &opts), "hello");
    }

    #[test]
    fn cut_fields_tab() {
        let opts = CutOptions {
            ranges: vec![Range::Single(2)],
            ..Default::default()
        };
        assert_eq!(cut_fields("a\tb\tc", &opts), Some("b".to_string()));
    }

    #[test]
    fn cut_fields_suppress() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1)],
            only_delimited: true,
            ..Default::default()
        };
        assert_eq!(cut_fields("no tabs", &opts), None);
    }

    #[test]
    fn cut_complement() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1), Range::Single(5)],
            complement: true,
            ..Default::default()
        };
        assert_eq!(cut_characters("abcde", &opts), "bcd");
    }

    #[test]
    fn cut_from_range() {
        let opts = CutOptions {
            ranges: vec![Range::From(3)],
            ..Default::default()
        };
        assert_eq!(cut_characters("abcde", &opts), "cde");
    }

    #[test]
    fn cut_to_range() {
        let opts = CutOptions {
            ranges: vec![Range::To(3)],
            ..Default::default()
        };
        assert_eq!(cut_characters("abcde", &opts), "abc");
    }
}
