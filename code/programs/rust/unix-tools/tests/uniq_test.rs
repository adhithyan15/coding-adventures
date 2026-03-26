//! # Integration Tests for uniq

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::uniq_tool::process_uniq;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("uniq.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load uniq.json");
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
    fn count_flag() {
        match parse_argv(&["uniq", "-c"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("count"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["uniq", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["uniq", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn basic_dedup() {
        assert_eq!(process_uniq("a\na\nb\n", false, false, false, false, 0, 0, 0), "a\nb\n");
    }

    #[test]
    fn repeated_only() {
        assert_eq!(process_uniq("a\na\nb\n", false, true, false, false, 0, 0, 0), "a\n");
    }

    #[test]
    fn unique_only() {
        assert_eq!(process_uniq("a\na\nb\n", false, false, true, false, 0, 0, 0), "b\n");
    }

    #[test]
    fn ignore_case() {
        assert_eq!(
            process_uniq("Apple\napple\nBANANA\n", false, false, false, true, 0, 0, 0),
            "Apple\nBANANA\n"
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(process_uniq("", false, false, false, false, 0, 0, 0), "");
    }
}
