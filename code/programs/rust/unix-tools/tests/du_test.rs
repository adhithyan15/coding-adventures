//! # Integration Tests for du
//!
//! These tests verify that the `du` JSON spec integrates correctly
//! with CLI Builder, and that the business logic correctly estimates
//! file space usage via recursive directory traversal.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::du_tool::{disk_usage, format_du_entry, DuEntry, DuOptions};
use std::path::PathBuf;
use std::fs;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("du.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load du.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn setup_test_dir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("failed to create temp dir");
    fs::write(dir.path().join("file1.txt"), "hello world\n").unwrap();
    fs::write(dir.path().join("file2.txt"), "test content\n").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("subdir/nested.txt"), "nested\n").unwrap();
    dir
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
        match parse_argv(&["du", "."]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["du", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("du")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["du", "--version"]) {
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
    fn du_directory() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let entries = disk_usage(&path, &DuOptions::default()).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn du_summarize() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { summarize: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn du_all_shows_files() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { all: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        let file_count = entries.iter().filter(|e| e.path.ends_with(".txt")).count();
        assert!(file_count > 0);
    }

    #[test]
    fn du_max_depth_zero() {
        let dir = setup_test_dir();
        let path = dir.path().to_string_lossy().to_string();
        let opts = DuOptions { max_depth: Some(0), ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn du_nonexistent_path() {
        assert!(disk_usage("/nonexistent/12345", &DuOptions::default()).is_err());
    }

    #[test]
    fn format_entry_default() {
        let entry = DuEntry { size_bytes: 4096, path: "test".into() };
        let output = format_du_entry(&entry, &DuOptions::default());
        assert!(output.contains("test"));
    }

    #[test]
    fn format_entry_human() {
        let entry = DuEntry { size_bytes: 1048576, path: "big".into() };
        let opts = DuOptions { human_readable: true, ..Default::default() };
        let output = format_du_entry(&entry, &opts);
        assert!(output.contains("1.0M"));
    }

    #[test]
    fn du_single_file() {
        let dir = setup_test_dir();
        let file = dir.path().join("file1.txt");
        let path = file.to_string_lossy().to_string();
        let opts = DuOptions { all: true, ..Default::default() };
        let entries = disk_usage(&path, &opts).unwrap();
        assert_eq!(entries.len(), 1);
    }
}
