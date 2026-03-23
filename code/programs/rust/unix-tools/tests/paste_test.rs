//! # Integration Tests for paste
//!
//! These tests verify that the `paste` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly merges
//! lines from multiple inputs.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::paste_tool::paste_lines;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("paste.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load paste.json");
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
        assert!(load_spec_from_file(&spec_path()).is_ok());
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
        match parse_argv(&["paste", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("paste")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["paste", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
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
    fn parallel_two_files() {
        let inputs = vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
        ];
        assert_eq!(paste_lines(&inputs, "\t", false), vec!["a\t1", "b\t2"]);
    }

    #[test]
    fn parallel_unequal() {
        let inputs = vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["1".into()],
        ];
        assert_eq!(paste_lines(&inputs, "\t", false), vec!["a\t1", "b\t", "c\t"]);
    }

    #[test]
    fn serial_mode() {
        let inputs = vec![
            vec!["a".into(), "b".into(), "c".into()],
        ];
        assert_eq!(paste_lines(&inputs, "\t", true), vec!["a\tb\tc"]);
    }

    #[test]
    fn custom_delimiter() {
        let inputs = vec![
            vec!["a".into()],
            vec!["b".into()],
        ];
        assert_eq!(paste_lines(&inputs, ":", false), vec!["a:b"]);
    }

    #[test]
    fn cycling_delimiters() {
        let inputs = vec![
            vec!["a".into()],
            vec!["b".into()],
            vec!["c".into()],
        ];
        assert_eq!(paste_lines(&inputs, ":,", false), vec!["a:b,c"]);
    }

    #[test]
    fn empty_input() {
        let inputs: Vec<Vec<String>> = vec![];
        assert!(paste_lines(&inputs, "\t", false).is_empty());
    }

    #[test]
    fn three_files_parallel() {
        let inputs = vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
            vec!["x".into(), "y".into()],
        ];
        assert_eq!(paste_lines(&inputs, "\t", false), vec!["a\t1\tx", "b\t2\ty"]);
    }
}
