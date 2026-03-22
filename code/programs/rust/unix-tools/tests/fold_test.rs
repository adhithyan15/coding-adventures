//! # Integration Tests for fold

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::fold_tool::fold_line;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("fold.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load fold.json");
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
        match parse_argv(&["fold", "-"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["fold", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["fold", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn fold_exact() {
        assert_eq!(fold_line("abcdefghij", 5, false, false), "abcde\nfghij");
    }

    #[test]
    fn fold_short() {
        assert_eq!(fold_line("abc", 10, false, false), "abc");
    }

    #[test]
    fn fold_empty() {
        assert_eq!(fold_line("", 10, false, false), "");
    }

    #[test]
    fn fold_three_segments() {
        assert_eq!(fold_line("abcdefghijklmno", 5, false, false), "abcde\nfghij\nklmno");
    }

    #[test]
    fn fold_with_spaces() {
        let result = fold_line("hello world foo", 10, true, false);
        assert!(result.contains('\n'));
    }
}
