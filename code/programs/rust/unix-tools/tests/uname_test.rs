//! # Integration Tests for uname
//!
//! These tests verify that the `uname` JSON spec integrates correctly
//! with CLI Builder, and that the business logic returns valid system
//! information.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::uname_tool::{format_uname, get_system_info};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("uname.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load uname.json");
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
        match parse_argv(&["uname"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["uname", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("uname")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["uname", "--version"]) {
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

    // All tests call get_system_info() which uses libc::uname — Unix only.

    #[cfg(unix)]
    #[test]
    fn system_info_succeeds() {
        assert!(get_system_info().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn sysname_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.sysname.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn default_shows_sysname() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, false, false, false, false, false, false, false, false);
        assert_eq!(output, info.sysname);
    }

    #[cfg(unix)]
    #[test]
    fn all_has_multiple_parts() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, true, false, false, false, false, false, false, false, false);
        let parts: Vec<&str> = output.split_whitespace().collect();
        assert!(parts.len() >= 3);
    }

    #[cfg(unix)]
    #[test]
    fn machine_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.machine.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn nodename_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.nodename.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn kernel_name_flag() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, true, false, false, false, false, false, false, false);
        assert_eq!(output, info.sysname);
    }

    #[cfg(unix)]
    #[test]
    fn machine_flag() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, false, false, false, false, true, false, false, false);
        assert_eq!(output, info.machine);
    }
}
