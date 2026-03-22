//! # Integration Tests for cat
//!
//! These tests exercise both the CLI Builder integration and the cat
//! business logic. The cat tool has more options than most other Unix
//! tools, so we test each option independently and in combination.
//!
//! ## Test Strategy
//!
//! - **Spec tests**: Verify the JSON spec loads and flags parse correctly.
//! - **Logic tests**: Verify `process_cat_content` transforms content
//!   correctly for each option and combination of options.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::cat_tool::{process_cat_content, CatOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("cat.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load cat.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads successfully
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "cat.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn no_flags_returns_parse() {
        match parse_argv(&["cat"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn number_flag() {
        match parse_argv(&["cat", "-n"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("number"),
                    Some(&serde_json::json!(true)),
                    "-n should set number to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn number_long_flag() {
        match parse_argv(&["cat", "--number"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("number"),
                    Some(&serde_json::json!(true)),
                    "--number should set number to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn squeeze_blank_flag() {
        match parse_argv(&["cat", "-s"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("squeeze_blank"),
                    Some(&serde_json::json!(true)),
                    "-s should set squeeze_blank to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn show_tabs_flag() {
        match parse_argv(&["cat", "-T"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("show_tabs"),
                    Some(&serde_json::json!(true)),
                    "-T should set show_tabs to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn show_ends_flag() {
        match parse_argv(&["cat", "-E"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("show_ends"),
                    Some(&serde_json::json!(true)),
                    "-E should set show_ends to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn show_all_flag() {
        match parse_argv(&["cat", "-A"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("show_all"),
                    Some(&serde_json::json!(true)),
                    "-A should set show_all to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["cat", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["cat", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — process_cat_content
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    /// Simple passthrough — no options, content comes out unchanged.
    #[test]
    fn simple_passthrough() {
        let opts = CatOptions::new();
        assert_eq!(
            process_cat_content("hello\nworld\n", &opts),
            "hello\nworld\n"
        );
    }

    /// Empty input produces empty output.
    #[test]
    fn empty_input() {
        let opts = CatOptions::new();
        assert_eq!(process_cat_content("", &opts), "");
    }

    /// Number all lines with -n.
    #[test]
    fn number_all_lines() {
        let mut opts = CatOptions::new();
        opts.number = true;
        let result = process_cat_content("a\nb\nc\n", &opts);
        assert!(result.contains("     1\ta"));
        assert!(result.contains("     2\tb"));
        assert!(result.contains("     3\tc"));
    }

    /// Number only non-blank lines with -b.
    #[test]
    fn number_nonblank_lines() {
        let mut opts = CatOptions::new();
        opts.number_nonblank = true;
        let result = process_cat_content("a\n\nb\n", &opts);
        assert!(result.contains("     1\ta"));
        assert!(result.contains("     2\tb"));
        // Blank line should not have a number
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "", "blank line should have no number prefix");
    }

    /// -b overrides -n for blank lines.
    #[test]
    fn number_nonblank_overrides_number() {
        let mut opts = CatOptions::new();
        opts.number = true;
        opts.number_nonblank = true;
        // -b should take precedence
        let result = process_cat_content("a\n\nb\n", &opts);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "", "blank line should not be numbered when -b is set");
    }

    /// Squeeze consecutive blank lines with -s.
    #[test]
    fn squeeze_blank_lines() {
        let mut opts = CatOptions::new();
        opts.squeeze_blank = true;
        let result = process_cat_content("a\n\n\n\nb\n", &opts);
        assert_eq!(result, "a\n\nb\n");
    }

    /// A single blank line is not squeezed.
    #[test]
    fn single_blank_not_squeezed() {
        let mut opts = CatOptions::new();
        opts.squeeze_blank = true;
        let result = process_cat_content("a\n\nb\n", &opts);
        assert_eq!(result, "a\n\nb\n");
    }

    /// Show tabs as ^I with -T.
    #[test]
    fn show_tabs() {
        let mut opts = CatOptions::new();
        opts.show_tabs = true;
        let result = process_cat_content("hello\tworld\n", &opts);
        assert_eq!(result, "hello^Iworld\n");
    }

    /// Show $ at end of each line with -E.
    #[test]
    fn show_ends() {
        let mut opts = CatOptions::new();
        opts.show_ends = true;
        let result = process_cat_content("hello\nworld\n", &opts);
        assert_eq!(result, "hello$\nworld$\n");
    }

    /// Show ends on blank lines — should show just "$".
    #[test]
    fn show_ends_blank_line() {
        let mut opts = CatOptions::new();
        opts.show_ends = true;
        let result = process_cat_content("a\n\nb\n", &opts);
        assert_eq!(result, "a$\n$\nb$\n");
    }

    /// The -A flag enables -v, -E, and -T simultaneously.
    #[test]
    fn show_all_sets_three_flags() {
        let mut opts = CatOptions::new();
        opts.apply_show_all();
        assert!(opts.show_nonprinting);
        assert!(opts.show_ends);
        assert!(opts.show_tabs);
    }

    /// Combined: number and show ends.
    #[test]
    fn number_and_show_ends() {
        let mut opts = CatOptions::new();
        opts.number = true;
        opts.show_ends = true;
        let result = process_cat_content("hello\n", &opts);
        assert!(result.contains("     1\thello$"));
    }

    /// Combined: squeeze and number.
    #[test]
    fn squeeze_and_number() {
        let mut opts = CatOptions::new();
        opts.squeeze_blank = true;
        opts.number = true;
        let result = process_cat_content("a\n\n\nb\n", &opts);
        // Should squeeze to one blank, then number the remaining lines
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3); // a, blank, b
    }

    /// Multiple tabs on one line.
    #[test]
    fn multiple_tabs() {
        let mut opts = CatOptions::new();
        opts.show_tabs = true;
        let result = process_cat_content("a\tb\tc\n", &opts);
        assert_eq!(result, "a^Ib^Ic\n");
    }
}
