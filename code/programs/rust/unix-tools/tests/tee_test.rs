//! # Integration Tests for tee
//!
//! These tests exercise both the CLI Builder integration and the tee
//! business logic. We test writing to single and multiple files,
//! append mode, overwrite mode, and error handling.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::tee_tool::tee_content;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("tee.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load tee.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn temp_path(name: &str) -> String {
    let dir = std::env::temp_dir();
    dir.join(format!("tee_integration_{}", name))
        .to_string_lossy()
        .into_owned()
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
        assert!(spec.is_ok(), "tee.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn no_flags_returns_parse() {
        match parse_argv(&["tee"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn append_flag() {
        match parse_argv(&["tee", "-a", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("append"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["tee", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["tee", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — tee_content
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn write_to_single_file() {
        let path = temp_path("int_single");
        tee_content("hello\n", &[path.clone()], false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn write_to_multiple_files() {
        let p1 = temp_path("int_m1");
        let p2 = temp_path("int_m2");
        tee_content("data\n", &[p1.clone(), p2.clone()], false).unwrap();
        assert_eq!(fs::read_to_string(&p1).unwrap(), "data\n");
        assert_eq!(fs::read_to_string(&p2).unwrap(), "data\n");
        fs::remove_file(&p1).ok();
        fs::remove_file(&p2).ok();
    }

    #[test]
    fn append_mode() {
        let path = temp_path("int_append");
        tee_content("first\n", &[path.clone()], false).unwrap();
        tee_content("second\n", &[path.clone()], true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn overwrite_mode() {
        let path = temp_path("int_overwrite");
        tee_content("original\n", &[path.clone()], false).unwrap();
        tee_content("replaced\n", &[path.clone()], false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "replaced\n");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn empty_file_list() {
        assert!(tee_content("data\n", &[], false).is_ok());
    }

    #[test]
    fn invalid_path_returns_error() {
        let result = tee_content("data\n", &["/nonexistent/dir/file.txt".into()], false);
        assert!(result.is_err());
    }
}
