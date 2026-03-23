//! # Integration Tests for unexpand

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::unexpand_tool::unexpand_line;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("unexpand.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load unexpand.json");
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
        match parse_argv(&["unexpand", "-"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["unexpand", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["unexpand", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn unexpand_leading_spaces() {
        assert_eq!(unexpand_line("        hello", &[8], false), "\thello");
    }

    #[test]
    fn unexpand_partial_spaces() {
        assert_eq!(unexpand_line("   hello", &[8], false), "   hello");
    }

    #[test]
    fn unexpand_no_spaces() {
        assert_eq!(unexpand_line("hello", &[8], false), "hello");
    }

    #[test]
    fn unexpand_custom_width() {
        assert_eq!(unexpand_line("    hello", &[4], false), "\thello");
    }
}
