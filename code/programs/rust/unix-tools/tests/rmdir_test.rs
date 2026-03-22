//! # Integration Tests for rmdir

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::rmdir_tool::{remove_directory, remove_with_parents};
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("rmdir.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load rmdir.json");
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
        match parse_argv(&["rmdir", "-p", "dir"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("parents"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["rmdir", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["rmdir", "--version"]) {
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
    fn remove_empty() {
        let dir = std::env::temp_dir().join("rmdir_int_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().into_owned();
        assert!(remove_directory(&path).is_ok());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn remove_non_empty_fails() {
        let dir = std::env::temp_dir().join("rmdir_int_nonempty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), "data").unwrap();
        let path = dir.to_string_lossy().into_owned();
        assert!(remove_directory(&path).is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
