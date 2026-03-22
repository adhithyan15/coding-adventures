//! # Integration Tests for dirname
//!
//! These tests exercise both the CLI Builder integration and the dirname
//! business logic. We test directory extraction for various path formats
//! including absolute, relative, root, and edge cases.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::dirname_tool::strip_dirname;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("dirname.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load dirname.json");
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
        assert!(spec.is_ok(), "dirname.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn with_name_arg() {
        match parse_argv(&["dirname", "/usr/bin/sort"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn zero_flag() {
        match parse_argv(&["dirname", "-z", "/usr/bin"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("zero"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["dirname", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["dirname", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — strip_dirname
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn simple_path() {
        assert_eq!(strip_dirname("/usr/bin/sort"), "/usr/bin");
    }

    #[test]
    fn trailing_slash() {
        assert_eq!(strip_dirname("/usr/bin/"), "/usr");
    }

    #[test]
    fn bare_filename() {
        assert_eq!(strip_dirname("hello"), ".");
    }

    #[test]
    fn root_path() {
        assert_eq!(strip_dirname("/"), "/");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_dirname(""), ".");
    }

    #[test]
    fn relative_path() {
        assert_eq!(strip_dirname("a/b"), "a");
    }

    #[test]
    fn file_in_root() {
        assert_eq!(strip_dirname("/hello"), "/");
    }

    #[test]
    fn deep_path() {
        assert_eq!(strip_dirname("/a/b/c/d"), "/a/b/c");
    }
}
