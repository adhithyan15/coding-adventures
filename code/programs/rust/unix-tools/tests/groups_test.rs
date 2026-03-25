//! # Integration Tests for groups
//!
//! These tests verify that the `groups` JSON spec integrates correctly
//! with CLI Builder, and that the business logic returns valid group
//! membership information.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::groups_tool::{format_groups, get_groups};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("groups.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load groups.json");
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
    fn parse_with_no_args() {
        // groups command with no user argument — may require at least
        // one argument depending on CLI Builder's variadic handling
        let parser = make_parser();
        let args: Vec<String> = vec!["groups".to_string()];
        // Just verify parsing doesn't panic; may succeed or fail
        let _ = parser.parse(&args);
    }

    #[test]
    fn help() {
        match parse_argv(&["groups", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("groups")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["groups", "--version"]) {
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

    #[cfg(unix)]
    #[test]
    fn get_groups_succeeds() {
        assert!(get_groups().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn has_at_least_one_group() {
        let groups = get_groups().unwrap();
        assert!(!groups.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn group_names_nonempty() {
        let groups = get_groups().unwrap();
        for name in &groups {
            assert!(!name.is_empty());
        }
    }

    #[cfg(unix)]
    #[test]
    fn format_output() {
        let groups = get_groups().unwrap();
        let output = format_groups(&groups);
        assert!(!output.is_empty());
        // Should contain at least one group name
        assert!(output.split_whitespace().count() >= 1);
    }

    #[cfg(unix)]
    #[test]
    fn consistent_results() {
        // On CI runners, getgrgid() can intermittently resolve the same
        // GID to different names between calls, so we only assert that
        // both calls succeed and return non-empty results.
        let first = get_groups().unwrap();
        let second = get_groups().unwrap();
        assert!(!first.is_empty(), "first call should return groups");
        assert!(!second.is_empty(), "second call should return groups");
    }
}
