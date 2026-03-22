//! # Integration Tests for expand

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::expand_tool::{expand_line, parse_tab_stops, next_tab_stop};
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("expand.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load expand.json");
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
    fn no_flags_returns_parse() {
        // Pass a file argument since the spec requires at least one FILE.
        match parse_argv(&["expand", "-"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["expand", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["expand", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn expand_tab_at_start() {
        assert_eq!(expand_line("\thello", &[8], false), "        hello");
    }

    #[test]
    fn expand_tab_custom_width() {
        assert_eq!(expand_line("\thello", &[4], false), "    hello");
    }

    #[test]
    fn expand_initial_only() {
        assert_eq!(expand_line("\thello\tworld", &[4], true), "    hello\tworld");
    }

    #[test]
    fn parse_default_stops() {
        assert_eq!(parse_tab_stops(""), Ok(vec![8]));
    }

    #[test]
    fn next_stop_regular() {
        assert_eq!(next_tab_stop(0, &[8]), 8);
        assert_eq!(next_tab_stop(3, &[8]), 5);
    }
}
