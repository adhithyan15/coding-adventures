//! # Integration Tests for basename
//!
//! These tests exercise both the CLI Builder integration and the basename
//! business logic. We test directory stripping, suffix removal, and
//! edge cases like root paths and empty strings.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::basename_tool::strip_basename;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("basename.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load basename.json");
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
        assert!(spec.is_ok(), "basename.json should load successfully");
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
        match parse_argv(&["basename", "/usr/bin/sort"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn multiple_flag() {
        match parse_argv(&["basename", "-a", "file1", "file2"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("multiple"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn suffix_flag() {
        match parse_argv(&["basename", "-s", ".txt", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("suffix").and_then(|v| v.as_str()),
                    Some(".txt")
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["basename", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["basename", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — strip_basename
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn simple_path() {
        assert_eq!(strip_basename("/usr/bin/sort", None), "sort");
    }

    #[test]
    fn with_suffix() {
        assert_eq!(strip_basename("program.sh", Some(".sh")), "program");
    }

    #[test]
    fn suffix_not_matching() {
        assert_eq!(strip_basename("program.sh", Some(".txt")), "program.sh");
    }

    #[test]
    fn root_path() {
        assert_eq!(strip_basename("/", None), "/");
    }

    #[test]
    fn trailing_slash() {
        assert_eq!(strip_basename("/usr/bin/", None), "bin");
    }

    #[test]
    fn no_directory() {
        assert_eq!(strip_basename("hello", None), "hello");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_basename("", None), "");
    }

    #[test]
    fn deep_path_with_suffix() {
        assert_eq!(
            strip_basename("/home/user/docs/report.pdf", Some(".pdf")),
            "report"
        );
    }
}
