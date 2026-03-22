//! # Integration Tests for id
//!
//! These tests verify that the `id` JSON spec integrates correctly
//! with CLI Builder, and that the business logic returns valid
//! user and group identity information.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::id_tool::{format_groups, format_group_id, format_id_full, format_user_id, get_user_info};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("id.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load id.json");
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
    fn default_parse() {
        match parse_argv(&["id"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["id", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("id")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["id", "--version"]) {
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
    fn get_info_succeeds() {
        assert!(get_user_info().is_ok());
    }

    #[test]
    fn username_nonempty() {
        let info = get_user_info().unwrap();
        assert!(!info.username.is_empty());
    }

    #[test]
    fn full_format_contains_uid() {
        let info = get_user_info().unwrap();
        let output = format_id_full(&info);
        assert!(output.contains("uid="));
        assert!(output.contains("gid="));
    }

    #[test]
    fn user_id_numeric() {
        let info = get_user_info().unwrap();
        let output = format_user_id(&info, false, false);
        assert!(output.parse::<u32>().is_ok());
    }

    #[test]
    fn user_id_name() {
        let info = get_user_info().unwrap();
        let output = format_user_id(&info, true, false);
        assert_eq!(output, info.username);
    }

    #[test]
    fn group_id_numeric() {
        let info = get_user_info().unwrap();
        let output = format_group_id(&info, false, false);
        assert!(output.parse::<u32>().is_ok());
    }

    #[test]
    fn groups_nonempty() {
        let info = get_user_info().unwrap();
        let output = format_groups(&info, false);
        assert!(!output.is_empty());
    }

    #[test]
    fn groups_names() {
        let info = get_user_info().unwrap();
        let output = format_groups(&info, true);
        assert!(!output.is_empty());
    }

    #[test]
    fn has_supplementary_groups() {
        let info = get_user_info().unwrap();
        assert!(!info.groups.is_empty());
    }
}
