//! # Integration Tests for realpath

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::realpath_tool::resolve_path;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("realpath.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load realpath.json");
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
    fn canonicalize_existing() {
        match parse_argv(&["realpath", "-e", "."]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("canonicalize_existing"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["realpath", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["realpath", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolve_current_dir() {
        let result = resolve_path(".", false, false, false);
        assert!(result.is_ok());
        assert!(Path::new(&result.unwrap()).is_absolute());
    }

    #[test]
    fn resolve_missing_with_m() {
        let result = resolve_path("/nonexistent/path", false, true, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/nonexistent/path");
    }

    #[test]
    fn resolve_missing_with_e_fails() {
        let result = resolve_path("/nonexistent/path", true, false, false);
        assert!(result.is_err());
    }
}
