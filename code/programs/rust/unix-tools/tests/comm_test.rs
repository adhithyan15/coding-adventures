//! # Integration Tests for comm
//!
//! These tests verify that the `comm` JSON spec integrates correctly
//! with CLI Builder, and that the merge-join algorithm produces
//! correct three-column output.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::comm_tool::compare_sorted;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("comm.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load comm.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
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
        match parse_argv(&["comm", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("comm")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["comm", "--version"]) {
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
    fn three_columns() {
        let f1 = s(&["apple", "cherry"]);
        let f2 = s(&["banana", "cherry"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["apple", "\tbanana", "\t\tcherry"]);
    }

    #[test]
    fn suppress_col1() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [true, false, false], "\t");
        assert_eq!(result, vec!["\tb", "c"]);
    }

    #[test]
    fn suppress_col2() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, true, false], "\t");
        assert_eq!(result, vec!["a", "\tb"]);
    }

    #[test]
    fn suppress_col3() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, false, true], "\t");
        assert_eq!(result, vec!["a", "\tc"]);
    }

    #[test]
    fn common_only() {
        let f1 = s(&["a", "b", "c"]);
        let f2 = s(&["b", "d"]);
        let result = compare_sorted(&f1, &f2, [true, true, false], "\t");
        assert_eq!(result, vec!["b"]);
    }

    #[test]
    fn identical_files() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["a", "b"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["\t\ta", "\t\tb"]);
    }

    #[test]
    fn no_common() {
        let f1 = s(&["a", "c"]);
        let f2 = s(&["b", "d"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["a", "\tb", "c", "\td"]);
    }

    #[test]
    fn empty_inputs() {
        let f1: Vec<String> = vec![];
        let f2: Vec<String> = vec![];
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert!(result.is_empty());
    }
}
