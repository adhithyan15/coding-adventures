//! # Integration Tests for wc
//!
//! These tests exercise both the CLI Builder integration and the wc
//! business logic. We test counting accuracy for lines, words, bytes,
//! and characters, as well as formatting output.
//!
//! ## Test Strategy
//!
//! - **Spec tests**: Verify the JSON spec loads and flags parse correctly.
//! - **Logic tests**: Verify `count_content` produces accurate counts for
//!   various inputs, including edge cases like empty files, UTF-8 content,
//!   and files without trailing newlines.
//! - **Format tests**: Verify `format_counts` produces correctly formatted
//!   output with the right columns selected.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::wc_tool::{count_content, format_counts, WcCounts, WcOptions};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("wc.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load wc.json");
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
        assert!(spec.is_ok(), "wc.json should load successfully");
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
        match parse_argv(&["wc"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn lines_flag() {
        match parse_argv(&["wc", "-l"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("lines"),
                    Some(&serde_json::json!(true)),
                    "-l should set lines to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn words_flag() {
        match parse_argv(&["wc", "-w"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("words"),
                    Some(&serde_json::json!(true)),
                    "-w should set words to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn bytes_flag() {
        match parse_argv(&["wc", "-c"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("bytes"),
                    Some(&serde_json::json!(true)),
                    "-c should set bytes to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn chars_flag() {
        match parse_argv(&["wc", "-m"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("chars"),
                    Some(&serde_json::json!(true)),
                    "-m should set chars to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn max_line_length_flag() {
        match parse_argv(&["wc", "-L"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("max_line_length"),
                    Some(&serde_json::json!(true)),
                    "-L should set max_line_length to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn long_flags() {
        match parse_argv(&["wc", "--lines", "--words"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("lines"), Some(&serde_json::json!(true)));
                assert_eq!(result.flags.get("words"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["wc", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["wc", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — count_content
// ---------------------------------------------------------------------------

#[cfg(test)]
mod counting {
    use super::*;

    /// Empty content should produce all-zero counts.
    #[test]
    fn empty_content() {
        let counts = count_content("");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.words, 0);
        assert_eq!(counts.bytes, 0);
        assert_eq!(counts.chars, 0);
        assert_eq!(counts.max_line_length, 0);
    }

    /// A single word without a newline.
    #[test]
    fn single_word_no_newline() {
        let counts = count_content("hello");
        assert_eq!(counts.words, 1);
        assert_eq!(counts.lines, 0); // no newline = no line counted
        assert_eq!(counts.bytes, 5);
        assert_eq!(counts.chars, 5);
    }

    /// A single line with a newline.
    #[test]
    fn single_line_with_newline() {
        let counts = count_content("hello\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 6); // 5 + newline
        assert_eq!(counts.chars, 6);
    }

    /// Multiple words on one line.
    #[test]
    fn multiple_words_one_line() {
        let counts = count_content("hello world\n");
        assert_eq!(counts.words, 2);
        assert_eq!(counts.lines, 1);
    }

    /// Multiple lines.
    #[test]
    fn multiple_lines() {
        let counts = count_content("one\ntwo\nthree\n");
        assert_eq!(counts.lines, 3);
        assert_eq!(counts.words, 3);
    }

    /// Blank lines contribute to line count but not word count.
    #[test]
    fn blank_lines() {
        let counts = count_content("\n\n\n");
        assert_eq!(counts.lines, 3);
        assert_eq!(counts.words, 0);
    }

    /// Leading and trailing whitespace.
    #[test]
    fn leading_trailing_whitespace() {
        let counts = count_content("  hello  \n");
        assert_eq!(counts.words, 1);
    }

    /// Multiple spaces between words.
    #[test]
    fn multiple_spaces() {
        let counts = count_content("a   b   c\n");
        assert_eq!(counts.words, 3);
    }

    /// Tabs count as whitespace (word separators).
    #[test]
    fn tabs_separate_words() {
        let counts = count_content("a\tb\tc\n");
        assert_eq!(counts.words, 3);
    }

    /// Max line length tracks the longest line.
    #[test]
    fn max_line_length() {
        let counts = count_content("short\na much longer line\nhi\n");
        assert_eq!(counts.max_line_length, 18); // "a much longer line"
    }

    /// Max line length for content without trailing newline.
    #[test]
    fn max_line_length_no_trailing_newline() {
        let counts = count_content("short\nthe longest line here");
        assert_eq!(counts.max_line_length, 21); // "the longest line here"
    }

    /// UTF-8: chars and bytes differ for multi-byte characters.
    #[test]
    fn utf8_multibyte() {
        // Each hiragana character is 3 bytes in UTF-8
        let content = "\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}\n"; // konnichiwa
        let counts = count_content(content);
        assert_eq!(counts.chars, 6); // 5 chars + newline
        assert_eq!(counts.bytes, 16); // 5 * 3 + 1
        assert_eq!(counts.words, 1);
        assert_eq!(counts.lines, 1);
    }

    /// Single newline: one line, zero words.
    #[test]
    fn just_newline() {
        let counts = count_content("\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 0);
        assert_eq!(counts.bytes, 1);
    }
}

// ---------------------------------------------------------------------------
// Test: Formatting — format_counts
// ---------------------------------------------------------------------------

#[cfg(test)]
mod formatting {
    use super::*;

    /// Default display (no flags): lines, words, bytes.
    #[test]
    fn default_triple() {
        let counts = WcCounts {
            lines: 3,
            words: 5,
            bytes: 28,
            chars: 28,
            max_line_length: 10,
        };
        let opts = WcOptions::new();
        let result = format_counts(&counts, &opts, Some("test.txt"), 1);
        assert_eq!(result, "3 5 28 test.txt");
    }

    /// Lines only.
    #[test]
    fn lines_only() {
        let counts = WcCounts {
            lines: 42,
            words: 100,
            bytes: 500,
            chars: 500,
            max_line_length: 80,
        };
        let mut opts = WcOptions::new();
        opts.show_lines = true;
        let result = format_counts(&counts, &opts, Some("file.txt"), 1);
        assert_eq!(result, "42 file.txt");
    }

    /// Words only.
    #[test]
    fn words_only() {
        let counts = WcCounts {
            lines: 10,
            words: 42,
            bytes: 200,
            chars: 200,
            max_line_length: 50,
        };
        let mut opts = WcOptions::new();
        opts.show_words = true;
        let result = format_counts(&counts, &opts, None, 1);
        assert_eq!(result, "42");
    }

    /// Bytes only.
    #[test]
    fn bytes_only() {
        let counts = WcCounts {
            lines: 10,
            words: 42,
            bytes: 200,
            chars: 180,
            max_line_length: 50,
        };
        let mut opts = WcOptions::new();
        opts.show_bytes = true;
        let result = format_counts(&counts, &opts, None, 1);
        assert_eq!(result, "200");
    }

    /// Chars only.
    #[test]
    fn chars_only() {
        let counts = WcCounts {
            lines: 10,
            words: 42,
            bytes: 200,
            chars: 180,
            max_line_length: 50,
        };
        let mut opts = WcOptions::new();
        opts.show_chars = true;
        let result = format_counts(&counts, &opts, None, 1);
        assert_eq!(result, "180");
    }

    /// Max line length only.
    #[test]
    fn max_line_length_only() {
        let counts = WcCounts {
            lines: 10,
            words: 42,
            bytes: 200,
            chars: 200,
            max_line_length: 75,
        };
        let mut opts = WcOptions::new();
        opts.show_max_line_length = true;
        let result = format_counts(&counts, &opts, None, 1);
        assert_eq!(result, "75");
    }

    /// Multiple flags: lines and words.
    #[test]
    fn lines_and_words() {
        let counts = WcCounts {
            lines: 5,
            words: 20,
            bytes: 100,
            chars: 100,
            max_line_length: 30,
        };
        let mut opts = WcOptions::new();
        opts.show_lines = true;
        opts.show_words = true;
        let result = format_counts(&counts, &opts, Some("f.txt"), 1);
        assert_eq!(result, "5 20 f.txt");
    }

    /// No filename: output ends after the last count.
    #[test]
    fn no_filename() {
        let counts = WcCounts {
            lines: 1,
            words: 2,
            bytes: 10,
            chars: 10,
            max_line_length: 5,
        };
        let opts = WcOptions::new();
        let result = format_counts(&counts, &opts, None, 1);
        assert_eq!(result, "1 2 10");
    }

    /// Field width padding for multiple files.
    #[test]
    fn field_width_padding() {
        let counts = WcCounts {
            lines: 3,
            words: 5,
            bytes: 28,
            chars: 28,
            max_line_length: 10,
        };
        let opts = WcOptions::new();
        let result = format_counts(&counts, &opts, Some("test.txt"), 7);
        assert_eq!(result, "      3       5      28 test.txt");
    }
}
