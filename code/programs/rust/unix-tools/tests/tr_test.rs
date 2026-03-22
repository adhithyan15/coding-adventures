//! # Integration Tests for tr

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::tr_tool::{translate, delete_chars, squeeze_repeats, expand_set};
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("tr.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load tr.json");
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
    fn delete_flag() {
        match parse_argv(&["tr", "-d", "abc"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("delete"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["tr", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["tr", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn translate_simple() {
        assert_eq!(translate("hello", "helo", "HELO", false), "HELLO");
    }

    #[test]
    fn delete_vowels() {
        assert_eq!(delete_chars("hello world", "aeiou", false), "hll wrld");
    }

    #[test]
    fn squeeze() {
        assert_eq!(squeeze_repeats("aabbcc", "a-c"), "abc");
    }

    #[test]
    fn expand_range() {
        assert_eq!(expand_set("a-d"), vec!['a', 'b', 'c', 'd']);
    }
}
