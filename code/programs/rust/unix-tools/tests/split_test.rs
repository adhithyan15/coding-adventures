//! # Integration Tests for split
//!
//! These tests verify that the `split` JSON spec integrates correctly
//! with CLI Builder, and that the split business logic handles line
//! splitting, byte splitting, chunk splitting, and suffix generation.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::split_tool::{split_by_lines, split_by_bytes, split_into_chunks, SplitOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("split.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load split.json");
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
        assert!(spec.is_ok(), "split.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_basic_split() {
        match parse_argv(&["split", "file.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["split", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("split"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["split", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_numeric_suffixes() {
        match parse_argv(&["split", "-d", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("numeric_suffixes"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_lines() {
        match parse_argv(&["split", "-l", "50", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("lines"), Some(&serde_json::json!(50)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_suffix_length() {
        match parse_argv(&["split", "-a", "3", "file.txt"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("suffix_length"), Some(&serde_json::json!(3)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — split_by_lines
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic_lines {
    use super::*;

    #[test]
    fn split_basic() {
        let content = "a\nb\nc\nd\ne";
        let result = split_by_lines(content, 2, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "a\nb".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "c\nd".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "e".to_string()));
    }

    #[test]
    fn split_exact_division() {
        let content = "a\nb\nc\nd";
        let result = split_by_lines(content, 2, "x", &SplitOptions::default());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "a\nb");
        assert_eq!(result[1].1, "c\nd");
    }

    #[test]
    fn split_one_line_per_file() {
        let content = "a\nb\nc";
        let result = split_by_lines(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "a".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "b".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "c".to_string()));
    }

    #[test]
    fn split_all_in_one() {
        let content = "a\nb\nc";
        let result = split_by_lines(content, 100, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "a\nb\nc");
    }

    #[test]
    fn split_empty_content() {
        let result = split_by_lines("", 5, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_zero_lines() {
        let result = split_by_lines("a\nb\nc", 0, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_custom_prefix() {
        let content = "a\nb";
        let result = split_by_lines(content, 1, "output_", &SplitOptions::default());
        assert_eq!(result[0].0, "output_aa");
        assert_eq!(result[1].0, "output_ab");
    }

    #[test]
    fn split_numeric_suffixes() {
        let content = "a\nb\nc";
        let opts = SplitOptions { numeric_suffixes: true, ..Default::default() };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "x00");
        assert_eq!(result[1].0, "x01");
        assert_eq!(result[2].0, "x02");
    }

    #[test]
    fn split_additional_suffix() {
        let content = "a\nb";
        let opts = SplitOptions {
            additional_suffix: ".txt".to_string(),
            ..Default::default()
        };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "xaa.txt");
        assert_eq!(result[1].0, "xab.txt");
    }

    #[test]
    fn split_suffix_length_3() {
        let content = "a\nb";
        let opts = SplitOptions { suffix_length: 3, ..Default::default() };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "xaaa");
        assert_eq!(result[1].0, "xaab");
    }

    #[test]
    fn split_single_line() {
        let content = "hello";
        let result = split_by_lines(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "hello");
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — split_by_bytes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic_bytes {
    use super::*;

    #[test]
    fn split_bytes_basic() {
        let content = "abcdefgh";
        let result = split_by_bytes(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "abc".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "def".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "gh".to_string()));
    }

    #[test]
    fn split_bytes_empty() {
        let result = split_by_bytes("", 5, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_bytes_exact_division() {
        let content = "abcdef";
        let result = split_by_bytes(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "abc");
        assert_eq!(result[1].1, "def");
    }

    #[test]
    fn split_bytes_single_byte() {
        let content = "abc";
        let result = split_by_bytes(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].1, "a");
        assert_eq!(result[1].1, "b");
        assert_eq!(result[2].1, "c");
    }

    #[test]
    fn split_bytes_larger_than_content() {
        let content = "abc";
        let result = split_by_bytes(content, 100, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "abc");
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — split_into_chunks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic_chunks {
    use super::*;

    #[test]
    fn split_chunks_basic() {
        let content = "abcdefghij"; // 10 bytes
        let result = split_into_chunks(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        // 10 / 3 = 3 remainder 1, first chunk gets 4 bytes
        assert_eq!(result[0].1.len(), 4);
        assert_eq!(result[1].1.len(), 3);
        assert_eq!(result[2].1.len(), 3);
    }

    #[test]
    fn split_chunks_empty() {
        let result = split_into_chunks("", 3, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_chunks_single() {
        let content = "hello";
        let result = split_into_chunks(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "hello");
    }

    #[test]
    fn split_chunks_two() {
        let content = "abcdef"; // 6 bytes
        let result = split_into_chunks(content, 2, "x", &SplitOptions::default());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "abc");
        assert_eq!(result[1].1, "def");
    }

    #[test]
    fn split_chunks_equal() {
        let content = "abcdef"; // 6 bytes, 3 chunks of 2
        let result = split_into_chunks(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].1, "ab");
        assert_eq!(result[1].1, "cd");
        assert_eq!(result[2].1, "ef");
    }
}
