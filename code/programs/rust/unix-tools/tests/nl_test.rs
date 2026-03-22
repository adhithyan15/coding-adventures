//! # Integration Tests for nl

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::nl_tool::{should_number, format_line_number, number_lines};
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("nl.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load nl.json");
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
        match parse_argv(&["nl", "-"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["nl", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["nl", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn should_number_all() {
        assert!(should_number("hello", "a"));
        assert!(should_number("", "a"));
    }

    #[test]
    fn should_number_non_empty() {
        assert!(should_number("hello", "t"));
        assert!(!should_number("", "t"));
        assert!(!should_number("   ", "t"));
    }

    #[test]
    fn should_number_none() {
        assert!(!should_number("hello", "n"));
    }

    #[test]
    fn format_rn() {
        assert_eq!(format_line_number(1, 6, "rn"), "     1");
    }

    #[test]
    fn format_ln() {
        assert_eq!(format_line_number(1, 6, "ln"), "1     ");
    }

    #[test]
    fn format_rz() {
        assert_eq!(format_line_number(1, 6, "rz"), "000001");
    }

    #[test]
    fn number_default() {
        let result = number_lines("hello\nworld\n", "t", "rn", 6, 1, 1, "\t");
        assert!(result.contains("     1\thello"));
        assert!(result.contains("     2\tworld"));
    }

    #[test]
    fn number_all() {
        let result = number_lines("hello\n\nworld\n", "a", "rn", 6, 1, 1, "\t");
        assert!(result.contains("     1"));
        assert!(result.contains("     2"));
        assert!(result.contains("     3"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(number_lines("", "t", "rn", 6, 1, 1, "\t"), "");
    }
}
