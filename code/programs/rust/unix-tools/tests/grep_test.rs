//! # Integration Tests for grep
//!
//! These tests verify that the `grep` JSON spec integrates correctly
//! with CLI Builder, and that the pattern matching business logic
//! handles fixed strings, regex, inversion, and word/line matching.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::grep_tool::{grep_line, grep_content, grep_count, GrepOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("grep.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load grep.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "grep.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_basic_grep() {
        match parse_argv(&["grep", "pattern", "file.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["grep", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("grep"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["grep", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_ignore_case() {
        match parse_argv(&["grep", "-i", "pattern", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("ignore_case"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_count() {
        match parse_argv(&["grep", "-c", "pattern", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("count"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_line_number() {
        match parse_argv(&["grep", "-n", "pattern", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("line_number"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_invert_match() {
        match parse_argv(&["grep", "-v", "pattern", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("invert_match"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_fixed_strings() {
        match parse_argv(&["grep", "-F", "pattern", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("fixed_strings"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — grep_line
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic_line {
    use super::*;

    #[test]
    fn fixed_string_match() {
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert!(grep_line("hello world", "world", &opts));
        assert!(!grep_line("hello world", "xyz", &opts));
    }

    #[test]
    fn fixed_string_case_insensitive() {
        let opts = GrepOptions {
            fixed_strings: true,
            ignore_case: true,
            ..Default::default()
        };
        assert!(grep_line("Hello World", "hello", &opts));
        assert!(grep_line("HELLO", "hello", &opts));
    }

    #[test]
    fn regex_dot_matches_any_char() {
        let opts = GrepOptions::default();
        assert!(grep_line("cat", "c.t", &opts));
        assert!(grep_line("cot", "c.t", &opts));
        assert!(!grep_line("ct", "c.t", &opts));
    }

    #[test]
    fn regex_star_zero_or_more() {
        let opts = GrepOptions::default();
        assert!(grep_line("ct", "ca*t", &opts));
        assert!(grep_line("cat", "ca*t", &opts));
        assert!(grep_line("caat", "ca*t", &opts));
    }

    #[test]
    fn regex_caret_anchor() {
        let opts = GrepOptions::default();
        assert!(grep_line("hello world", "^hello", &opts));
        assert!(!grep_line("say hello", "^hello", &opts));
    }

    #[test]
    fn regex_dollar_anchor() {
        let opts = GrepOptions::default();
        assert!(grep_line("hello world", "world$", &opts));
        assert!(!grep_line("world hello", "world$", &opts));
    }

    #[test]
    fn regex_dot_star() {
        let opts = GrepOptions::default();
        assert!(grep_line("anything at all", "any.*all", &opts));
    }

    #[test]
    fn invert_match() {
        let opts = GrepOptions {
            invert_match: true,
            fixed_strings: true,
            ..Default::default()
        };
        assert!(!grep_line("hello world", "hello", &opts));
        assert!(grep_line("goodbye world", "hello", &opts));
    }

    #[test]
    fn line_regexp_fixed() {
        let opts = GrepOptions {
            line_regexp: true,
            fixed_strings: true,
            ..Default::default()
        };
        assert!(grep_line("hello", "hello", &opts));
        assert!(!grep_line("hello world", "hello", &opts));
    }

    #[test]
    fn line_regexp_regex() {
        let opts = GrepOptions { line_regexp: true, ..Default::default() };
        assert!(grep_line("cat", "c.t", &opts));
        assert!(!grep_line("concat", "c.t", &opts));
    }

    #[test]
    fn word_regexp() {
        let opts = GrepOptions {
            word_regexp: true,
            fixed_strings: true,
            ..Default::default()
        };
        assert!(grep_line("the cat sat", "cat", &opts));
        assert!(!grep_line("concatenate", "cat", &opts));
    }

    #[test]
    fn empty_pattern_matches_everything() {
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert!(grep_line("anything", "", &opts));
    }

    #[test]
    fn escaped_dot_literal() {
        let opts = GrepOptions::default();
        assert!(grep_line("file.txt", "file\\.txt", &opts));
        assert!(!grep_line("fileXtxt", "file\\.txt", &opts));
    }

    #[test]
    fn unanchored_match() {
        let opts = GrepOptions::default();
        assert!(grep_line("the cat sat", "cat", &opts));
        assert!(grep_line("catalog", "cat", &opts));
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — grep_content and grep_count
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic_content {
    use super::*;

    #[test]
    fn grep_content_basic() {
        let content = "apple\nbanana\ncherry\nbanana split";
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        let results = grep_content(content, "banana", &opts);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (2, "banana".to_string()));
        assert_eq!(results[1], (4, "banana split".to_string()));
    }

    #[test]
    fn grep_content_no_matches() {
        let content = "apple\nbanana\ncherry";
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        let results = grep_content(content, "xyz", &opts);
        assert!(results.is_empty());
    }

    #[test]
    fn grep_content_case_insensitive() {
        let content = "Apple\nBANANA\ncherry";
        let opts = GrepOptions {
            fixed_strings: true,
            ignore_case: true,
            ..Default::default()
        };
        let results = grep_content(content, "banana", &opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (2, "BANANA".to_string()));
    }

    #[test]
    fn grep_count_basic() {
        let content = "one\ntwo\nthree\ntwo again";
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert_eq!(grep_count(content, "two", &opts), 2);
    }

    #[test]
    fn grep_count_no_matches() {
        let content = "apple\nbanana";
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert_eq!(grep_count(content, "xyz", &opts), 0);
    }

    #[test]
    fn grep_count_with_invert() {
        let content = "apple\nbanana\ncherry";
        let opts = GrepOptions {
            fixed_strings: true,
            invert_match: true,
            ..Default::default()
        };
        assert_eq!(grep_count(content, "banana", &opts), 2);
    }

    #[test]
    fn grep_content_empty_input() {
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        let results = grep_content("", "pattern", &opts);
        assert!(results.is_empty());
    }

    #[test]
    fn grep_content_line_numbers_are_1_based() {
        let content = "first\nsecond\nthird";
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        let results = grep_content(content, "third", &opts);
        assert_eq!(results[0].0, 3);
    }
}
