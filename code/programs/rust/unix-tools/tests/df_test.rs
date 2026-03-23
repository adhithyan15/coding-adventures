//! # Integration Tests for df
//!
//! These tests verify that the `df` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly queries
//! file system disk space usage.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::df_tool::{bytes_to_1k_blocks, format_human_size, format_si_size, get_fs_info};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("df.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load df.json");
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
    fn parse_with_path() {
        match parse_argv(&["df", "/"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["df", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("df")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["df", "--version"]) {
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
    fn root_fs_info() {
        let info = get_fs_info("/");
        assert!(info.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn total_positive() {
        let info = get_fs_info("/").unwrap();
        assert!(info.total_bytes > 0);
    }

    #[cfg(unix)]
    #[test]
    fn use_percent_in_range() {
        let info = get_fs_info("/").unwrap();
        assert!((0.0..=100.0).contains(&info.use_percent));
    }

    #[test]
    fn invalid_path_errors() {
        assert!(get_fs_info("/nonexistent/12345").is_err());
    }

    #[test]
    fn human_format() {
        assert_eq!(format_human_size(1024), "1.0K");
        assert_eq!(format_human_size(1048576), "1.0M");
        assert_eq!(format_human_size(500), "500");
    }

    #[test]
    fn si_format() {
        assert_eq!(format_si_size(1000), "1.0k");
        assert_eq!(format_si_size(1_000_000), "1.0M");
    }

    #[test]
    fn block_conversion() {
        assert_eq!(bytes_to_1k_blocks(4096), 4);
        assert_eq!(bytes_to_1k_blocks(0), 0);
        assert_eq!(bytes_to_1k_blocks(500), 1);
    }

    #[cfg(unix)]
    #[test]
    fn block_size_positive() {
        let info = get_fs_info("/").unwrap();
        assert!(info.block_size > 0);
    }
}
