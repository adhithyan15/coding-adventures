//! # Integration Tests for mkdir
//!
//! These tests exercise both the CLI Builder integration and the mkdir
//! business logic.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::mkdir_tool::make_directory;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("mkdir.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load mkdir.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        assert!(load_spec_from_file(&spec_path()).is_ok());
    }
}

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn parents_flag() {
        match parse_argv(&["mkdir", "-p", "dir"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("parents"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["mkdir", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["mkdir", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn create_directory() {
        let dir = std::env::temp_dir().join("mkdir_int_test");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.to_string_lossy().into_owned();
        assert!(make_directory(&path, false).is_ok());
        assert!(Path::new(&path).is_dir());
        fs::remove_dir(&path).ok();
    }

    #[test]
    fn create_with_parents() {
        let dir = std::env::temp_dir().join("mkdir_int_p/a/b");
        let root = std::env::temp_dir().join("mkdir_int_p");
        let _ = fs::remove_dir_all(&root);
        let path = dir.to_string_lossy().into_owned();
        assert!(make_directory(&path, true).is_ok());
        assert!(Path::new(&path).is_dir());
        fs::remove_dir_all(&root).ok();
    }
}
