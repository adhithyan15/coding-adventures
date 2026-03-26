//! # Integration Tests for join
//!
//! These tests verify that the `join` JSON spec integrates correctly
//! with CLI Builder, and that the merge-join business logic handles
//! basic joins, unpaired lines, custom separators, and case-insensitive
//! matching.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::join_tool::{join_lines, JoinOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("join.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load join.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
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
        assert!(spec.is_ok(), "join.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_basic_join() {
        match parse_argv(&["join", "file1.txt", "file2.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["join", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("join"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["join", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_ignore_case() {
        match parse_argv(&["join", "-i", "f1", "f2"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("ignore_case"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_separator() {
        match parse_argv(&["join", "-t", ":", "f1", "f2"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("separator"), Some(&serde_json::json!(":")));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn basic_join() {
        let lines1 = s(&["1 Alice", "2 Bob"]);
        let lines2 = s(&["1 Engineering", "2 Marketing"]);
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(result, vec!["1 Alice Engineering", "2 Bob Marketing"]);
    }

    #[test]
    fn join_no_matches() {
        let lines1 = s(&["1 Alice", "3 Charlie"]);
        let lines2 = s(&["2 Engineering", "4 Sales"]);
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn join_partial_match() {
        let lines1 = s(&["1 Alice", "2 Bob", "3 Charlie"]);
        let lines2 = s(&["1 Engineering", "3 Sales"]);
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(result, vec!["1 Alice Engineering", "3 Charlie Sales"]);
    }

    #[test]
    fn unpaired_file1() {
        let lines1 = s(&["1 Alice", "2 Bob", "3 Charlie"]);
        let lines2 = s(&["1 Engineering", "3 Sales"]);
        let opts = JoinOptions {
            unpaired: vec![1],
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["1 Alice Engineering", "2 Bob", "3 Charlie Sales"]
        );
    }

    #[test]
    fn unpaired_file2() {
        let lines1 = s(&["1 Alice"]);
        let lines2 = s(&["1 Engineering", "2 Marketing"]);
        let opts = JoinOptions {
            unpaired: vec![2],
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["1 Alice Engineering", "2 Marketing"]);
    }

    #[test]
    fn only_unpaired() {
        let lines1 = s(&["1 Alice", "2 Bob"]);
        let lines2 = s(&["1 Engineering"]);
        let opts = JoinOptions {
            only_unpaired: Some(1),
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["2 Bob"]);
    }

    #[test]
    fn join_on_field2() {
        let lines1 = s(&["Alice 1", "Bob 2"]);
        let lines2 = s(&["Engineering 1", "Marketing 2"]);
        let opts = JoinOptions {
            field1: 2,
            field2: 2,
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["1 Alice Engineering", "2 Bob Marketing"]);
    }

    #[test]
    fn custom_separator() {
        let lines1 = s(&["1:Alice", "2:Bob"]);
        let lines2 = s(&["1:Engineering", "2:Marketing"]);
        let opts = JoinOptions {
            separator: Some(':'),
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["1:Alice:Engineering", "2:Bob:Marketing"]);
    }

    #[test]
    fn case_insensitive_join() {
        let lines1 = s(&["a Alice", "B Bob"]);
        let lines2 = s(&["A Engineering", "b Marketing"]);
        let opts = JoinOptions {
            ignore_case: true,
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["a Alice Engineering", "B Bob Marketing"]
        );
    }

    #[test]
    fn empty_inputs() {
        let empty: Vec<String> = vec![];
        let lines = s(&["1 Alice"]);
        assert!(join_lines(&empty, &lines, &JoinOptions::default()).is_empty());
        assert!(join_lines(&lines, &empty, &JoinOptions::default()).is_empty());
        assert!(join_lines(&empty, &empty, &JoinOptions::default()).is_empty());
    }

    #[test]
    fn many_to_many_join() {
        let lines1 = s(&["1 A", "1 B"]);
        let lines2 = s(&["1 X", "1 Y"]);
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(result, vec!["1 A X", "1 A Y", "1 B X", "1 B Y"]);
    }

    #[test]
    fn single_field_lines() {
        let lines1 = s(&["alpha", "beta"]);
        let lines2 = s(&["alpha", "gamma"]);
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        // Both have key "alpha" with no other fields
        assert_eq!(result, vec!["alpha"]);
    }

    #[test]
    fn unpaired_both_files() {
        let lines1 = s(&["1 Alice", "2 Bob"]);
        let lines2 = s(&["2 Marketing", "3 Sales"]);
        let opts = JoinOptions {
            unpaired: vec![1, 2],
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["1 Alice", "2 Bob Marketing", "3 Sales"]
        );
    }
}
