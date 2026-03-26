//! # Integration Tests for sort
//!
//! These tests verify that the `sort` JSON spec integrates correctly
//! with CLI Builder, and that the business logic produces correctly
//! sorted output under various modes and options.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::sort_tool::{sort_lines, SortOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("sort.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load sort.json");
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
        assert!(spec.is_ok(), "sort.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_with_file() {
        match parse_argv(&["sort", "file.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["sort", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("sort"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["sort", "--version"]) {
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
    fn alphabetical_sort() {
        let lines = vec!["cherry".into(), "apple".into(), "banana".into()];
        let result = sort_lines(&lines, &SortOptions::default());
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn numeric_sort() {
        let lines = vec!["10".into(), "2".into(), "1".into()];
        let opts = SortOptions { numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["1", "2", "10"]);
    }

    #[test]
    fn reverse_sort() {
        let lines = vec!["a".into(), "c".into(), "b".into()];
        let opts = SortOptions { reverse: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["c", "b", "a"]);
    }

    #[test]
    fn unique_sort() {
        let lines = vec!["a".into(), "b".into(), "a".into()];
        let opts = SortOptions { unique: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn case_insensitive_sort() {
        let lines = vec!["Banana".into(), "apple".into(), "Cherry".into()];
        let opts = SortOptions { ignore_case: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["apple", "Banana", "Cherry"]);
    }

    #[test]
    fn human_numeric_sort() {
        let lines = vec!["1G".into(), "2K".into(), "3M".into()];
        let opts = SortOptions { human_numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["2K", "3M", "1G"]);
    }

    #[test]
    fn month_sort() {
        let lines = vec!["DEC".into(), "JAN".into(), "JUN".into()];
        let opts = SortOptions { month: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["JAN", "JUN", "DEC"]);
    }

    #[test]
    fn version_sort() {
        let lines = vec!["v1.10".into(), "v1.2".into(), "v1.1".into()];
        let opts = SortOptions { version: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["v1.1", "v1.2", "v1.10"]);
    }

    #[test]
    fn empty_input() {
        let lines: Vec<String> = vec![];
        let result = sort_lines(&lines, &SortOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn single_line() {
        let lines = vec!["only".into()];
        let result = sort_lines(&lines, &SortOptions::default());
        assert_eq!(result, vec!["only"]);
    }
}
