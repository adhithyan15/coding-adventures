//! # wc — Word, Line, and Byte Count
//!
//! This module implements the business logic for the `wc` command.
//! The `wc` utility counts lines, words, bytes, and characters in
//! files or standard input.
//!
//! ## What Counts as a "Word"?
//!
//! A "word" is a maximal sequence of non-whitespace characters,
//! separated by whitespace. This matches the POSIX definition:
//!
//! ```text
//!     "hello world"   → 2 words
//!     "  hello  "     → 1 word
//!     ""              → 0 words
//!     "\n\n"          → 0 words
//! ```
//!
//! ## Lines vs Newlines
//!
//! The "line count" is actually a "newline count." A file that ends
//! without a trailing newline will have one fewer "line" than you
//! might expect. This matches POSIX behavior:
//!
//! ```text
//!     "hello\n"       → 1 line
//!     "hello"         → 0 lines  (no newline = no line counted)
//!     "a\nb\nc\n"     → 3 lines
//! ```
//!
//! ## Bytes vs Characters
//!
//! In ASCII, bytes and characters are the same. But in UTF-8, a single
//! character can span multiple bytes. The `-c` flag counts bytes; the
//! `-m` flag counts characters. They are mutually exclusive.

// ---------------------------------------------------------------------------
// Count result
// ---------------------------------------------------------------------------

/// The result of counting a single file (or stdin).
///
/// Each field corresponds to one column in wc's output. When no
/// specific flags are given, wc shows lines, words, and bytes
/// (the "default triple").
#[derive(Debug, Clone, PartialEq)]
pub struct WcCounts {
    /// Number of newline characters (`-l`)
    pub lines: usize,
    /// Number of words (`-w`)
    pub words: usize,
    /// Number of bytes (`-c`)
    pub bytes: usize,
    /// Number of characters (`-m`)
    pub chars: usize,
    /// Length of the longest line (`-L`)
    pub max_line_length: usize,
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Which counts to display in the output.
///
/// When none of these are true, wc defaults to showing lines, words,
/// and bytes — the classic "default triple."
#[derive(Debug, Clone)]
pub struct WcOptions {
    pub show_lines: bool,
    pub show_words: bool,
    pub show_bytes: bool,
    pub show_chars: bool,
    pub show_max_line_length: bool,
}

impl WcOptions {
    /// Create options with nothing selected (will trigger default behavior).
    pub fn new() -> Self {
        WcOptions {
            show_lines: false,
            show_words: false,
            show_bytes: false,
            show_chars: false,
            show_max_line_length: false,
        }
    }

    /// Check whether any specific flag was set.
    ///
    /// If no flags are set, the caller should use the default triple
    /// (lines, words, bytes).
    pub fn any_selected(&self) -> bool {
        self.show_lines
            || self.show_words
            || self.show_bytes
            || self.show_chars
            || self.show_max_line_length
    }
}

// ---------------------------------------------------------------------------
// Core counting function
// ---------------------------------------------------------------------------

/// Count lines, words, bytes, and characters in a string.
///
/// This is the pure counting function — no I/O, no formatting.
/// It returns all counts at once; the caller decides which to display
/// based on the flags.
///
/// ## Algorithm
///
/// We iterate through the content once, maintaining all counters
/// simultaneously. This is O(n) in the size of the content with
/// a single pass.
///
/// ```text
///     for each character:
///       - if it's a newline → increment line count, check max line length
///       - if it's whitespace after non-whitespace → we just ended a word
///       - always → increment char count, add byte length
/// ```
pub fn count_content(content: &str) -> WcCounts {
    let mut lines: usize = 0;
    let mut words: usize = 0;
    let mut chars: usize = 0;
    let bytes = content.len();
    let mut max_line_length: usize = 0;
    let mut current_line_length: usize = 0;

    // Track whether we're inside a word (a run of non-whitespace).
    // When we transition from inside a word to whitespace, that's
    // the end of a word.
    let mut in_word = false;

    for ch in content.chars() {
        chars += 1;

        if ch == '\n' {
            // --- Newline: increment line count ---
            lines += 1;

            // Check if this line is the longest so far.
            if current_line_length > max_line_length {
                max_line_length = current_line_length;
            }
            current_line_length = 0;

            // End of a word if we were in one
            if in_word {
                words += 1;
                in_word = false;
            }
        } else if ch.is_whitespace() {
            // --- Whitespace (not newline): might end a word ---
            // We don't count the character toward line length for
            // display purposes — tabs and spaces vary in display width,
            // but we use simple character count like GNU wc.
            current_line_length += 1;

            if in_word {
                words += 1;
                in_word = false;
            }
        } else {
            // --- Non-whitespace: we're inside a word ---
            current_line_length += 1;
            in_word = true;
        }
    }

    // --- Handle the last word ---
    // If the content doesn't end with whitespace, we're still "in a word"
    // when the loop ends. Count it.
    if in_word {
        words += 1;
    }

    // --- Handle the last line ---
    // Check if the final line (which might not end with \n) is the longest.
    if current_line_length > max_line_length {
        max_line_length = current_line_length;
    }

    WcCounts {
        lines,
        words,
        bytes,
        chars,
        max_line_length,
    }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Format a single file's counts into a display string.
///
/// The output format matches GNU `wc`: each selected count is right-
/// justified in a field, separated by spaces. The filename (if any)
/// appears at the end.
///
/// ## Field Width
///
/// GNU wc uses a minimum field width of 1 when there's a single file
/// and a wider width when there are multiple files. For simplicity,
/// we use a width of 1 when only one file is being counted, and a
/// width based on the largest number when multiple files are counted.
///
/// ```text
///     Single file:   "3 5 28 hello.txt"
///     Multiple:      "  3   5  28 hello.txt"
/// ```
pub fn format_counts(
    counts: &WcCounts,
    options: &WcOptions,
    filename: Option<&str>,
    field_width: usize,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Determine which counts to show. If no specific flags were set,
    // use the default triple: lines, words, bytes.
    let show_lines = options.show_lines || !options.any_selected();
    let show_words = options.show_words || !options.any_selected();
    let show_bytes = options.show_bytes || (!options.any_selected() && !options.show_chars);
    let show_chars = options.show_chars;
    let show_max = options.show_max_line_length;

    // Build the output fields in the canonical order:
    // lines, words, chars, bytes, max_line_length
    if show_lines {
        parts.push(format!("{:>width$}", counts.lines, width = field_width));
    }
    if show_words {
        parts.push(format!("{:>width$}", counts.words, width = field_width));
    }
    if show_chars {
        parts.push(format!("{:>width$}", counts.chars, width = field_width));
    }
    if show_bytes {
        parts.push(format!("{:>width$}", counts.bytes, width = field_width));
    }
    if show_max {
        parts.push(format!(
            "{:>width$}",
            counts.max_line_length,
            width = field_width
        ));
    }

    let mut result = parts.join(" ");

    // Append the filename if provided
    if let Some(name) = filename {
        result.push(' ');
        result.push_str(name);
    }

    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content() {
        let counts = count_content("");
        assert_eq!(
            counts,
            WcCounts {
                lines: 0,
                words: 0,
                bytes: 0,
                chars: 0,
                max_line_length: 0,
            }
        );
    }

    #[test]
    fn single_word() {
        let counts = count_content("hello");
        assert_eq!(counts.words, 1);
        assert_eq!(counts.lines, 0); // no newline
        assert_eq!(counts.bytes, 5);
        assert_eq!(counts.chars, 5);
    }

    #[test]
    fn single_line_with_newline() {
        let counts = count_content("hello\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 6);
        assert_eq!(counts.chars, 6);
    }

    #[test]
    fn multiple_words() {
        let counts = count_content("hello world\n");
        assert_eq!(counts.words, 2);
        assert_eq!(counts.lines, 1);
    }

    #[test]
    fn multiple_lines() {
        let counts = count_content("one\ntwo\nthree\n");
        assert_eq!(counts.lines, 3);
        assert_eq!(counts.words, 3);
    }

    #[test]
    fn blank_lines_have_no_words() {
        let counts = count_content("\n\n\n");
        assert_eq!(counts.lines, 3);
        assert_eq!(counts.words, 0);
    }

    #[test]
    fn max_line_length() {
        let counts = count_content("short\na longer line\nhi\n");
        assert_eq!(counts.max_line_length, 13); // "a longer line"
    }

    #[test]
    fn utf8_chars_vs_bytes() {
        // The emoji "hi" in Japanese: each character is 3 bytes in UTF-8
        let content = "\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}\n";
        let counts = count_content(content);
        assert_eq!(counts.chars, 6); // 5 chars + newline
        assert_eq!(counts.bytes, 16); // 5 * 3 bytes + 1 newline
    }

    #[test]
    fn leading_and_trailing_whitespace() {
        let counts = count_content("  hello  \n");
        assert_eq!(counts.words, 1);
    }

    #[test]
    fn format_default_triple() {
        let counts = WcCounts {
            lines: 3,
            words: 5,
            bytes: 28,
            chars: 28,
            max_line_length: 10,
        };
        let opts = WcOptions::new(); // no flags → default triple
        let result = format_counts(&counts, &opts, Some("test.txt"), 1);
        assert_eq!(result, "3 5 28 test.txt");
    }

    #[test]
    fn format_lines_only() {
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

    #[test]
    fn format_no_filename() {
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

    #[test]
    fn format_chars_flag() {
        let counts = WcCounts {
            lines: 1,
            words: 2,
            bytes: 20,
            chars: 10,
            max_line_length: 5,
        };
        let mut opts = WcOptions::new();
        opts.show_chars = true;
        let result = format_counts(&counts, &opts, None, 1);
        // Only chars should show
        assert_eq!(result, "10");
    }
}
