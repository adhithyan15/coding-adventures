//! # cat — Concatenate and Print Files
//!
//! This module implements the business logic for the `cat` command.
//! The `cat` utility reads files sequentially, writing their contents
//! to standard output. If no file is specified, or if the file is `-`,
//! it reads from standard input.
//!
//! ## Options Overview
//!
//! The GNU version of `cat` supports several display options:
//!
//! ```text
//!     -n       Number all output lines
//!     -b       Number only non-blank lines (overrides -n)
//!     -s       Squeeze consecutive blank lines into one
//!     -T       Show tabs as ^I
//!     -E       Show $ at end of each line
//!     -v       Show non-printing characters using ^ and M- notation
//!     -A       Equivalent to -vET (show all)
//! ```
//!
//! ## Design
//!
//! The core function `process_cat_content` takes a string of content
//! and a set of options, then returns the transformed output. This
//! keeps the I/O (reading files, writing stdout) separate from the
//! transformation logic, making everything testable.

use std::io::{self, Read};
use std::fs;

// ---------------------------------------------------------------------------
// Options struct
// ---------------------------------------------------------------------------

/// Configuration options for cat output transformation.
///
/// These correspond directly to the command-line flags defined in
/// `cat.json`. The `show_all` flag is expanded into its component
/// parts (`show_ends`, `show_tabs`, `show_nonprinting`) before being
/// stored here.
#[derive(Debug, Clone)]
pub struct CatOptions {
    /// Number all output lines (`-n`)
    pub number: bool,
    /// Number only non-blank lines, overrides `number` (`-b`)
    pub number_nonblank: bool,
    /// Squeeze consecutive blank lines into one (`-s`)
    pub squeeze_blank: bool,
    /// Display TAB characters as ^I (`-T`)
    pub show_tabs: bool,
    /// Display $ at end of each line (`-E`)
    pub show_ends: bool,
    /// Show non-printing characters in ^ and M- notation (`-v`)
    pub show_nonprinting: bool,
}

impl CatOptions {
    /// Create a new `CatOptions` with all options disabled.
    pub fn new() -> Self {
        CatOptions {
            number: false,
            number_nonblank: false,
            squeeze_blank: false,
            show_tabs: false,
            show_ends: false,
            show_nonprinting: false,
        }
    }

    /// Apply the `show_all` (`-A`) flag by enabling `-v`, `-E`, and `-T`.
    ///
    /// The `-A` flag is syntactic sugar — it's equivalent to passing
    /// all three flags individually. We expand it here so the rest of
    /// the code doesn't need to know about `-A`.
    pub fn apply_show_all(&mut self) {
        self.show_nonprinting = true;
        self.show_ends = true;
        self.show_tabs = true;
    }
}

// ---------------------------------------------------------------------------
// Core transformation
// ---------------------------------------------------------------------------

/// Transform file content according to the given cat options.
///
/// This is the pure business logic function. It takes a string of content
/// and returns the transformed output. No I/O happens here.
///
/// ## Line Processing Pipeline
///
/// Each line flows through these stages:
///
/// ```text
///     Input line
///       │
///       ├─► Squeeze blank?  (skip if consecutive blank and -s)
///       ├─► Show non-printing chars?  (transform if -v)
///       ├─► Show tabs?  (replace \t with ^I if -T)
///       ├─► Show ends?  (append $ if -E)
///       └─► Number?  (prepend line number if -n or -b)
///           │
///           ▼
///     Output line
/// ```
pub fn process_cat_content(content: &str, options: &CatOptions) -> String {
    let mut output = String::with_capacity(content.len());
    let mut line_number: usize = 1;
    let mut prev_blank = false;

    // We split on newline boundaries but preserve the structure.
    // Using `.split('\n')` gives us the lines without their terminators.
    // We need to re-add them (except possibly for the very last one
    // if the input doesn't end with a newline).
    let lines: Vec<&str> = content.split('\n').collect();
    let last_idx = if lines.last() == Some(&"") {
        // Content ended with \n — the split produces an empty trailing
        // element that we should not treat as a real line.
        lines.len() - 1
    } else {
        lines.len()
    };

    for (i, line) in lines.iter().enumerate() {
        if i >= last_idx {
            // This is the phantom empty string after a trailing newline.
            break;
        }

        let is_blank = line.is_empty();

        // --- Squeeze blank lines ---
        // If -s is set and we see consecutive blank lines, skip all but
        // the first one. This collapses runs of empty lines into a single
        // empty line.
        if options.squeeze_blank && is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;

        // --- Build the transformed line ---
        let mut transformed = String::new();

        // --- Line numbering ---
        // -b (number_nonblank) takes precedence over -n (number).
        // With -b, blank lines are not numbered.
        // With -n, all lines are numbered.
        if options.number_nonblank {
            if !is_blank {
                transformed.push_str(&format!("{:>6}\t", line_number));
                line_number += 1;
            } else {
                // Blank lines get no number, just the empty line
            }
        } else if options.number {
            transformed.push_str(&format!("{:>6}\t", line_number));
            line_number += 1;
        }

        // --- Transform the line content ---
        for ch in line.chars() {
            if options.show_tabs && ch == '\t' {
                // Show tabs as ^I (caret notation)
                transformed.push_str("^I");
            } else if options.show_nonprinting && ch != '\t' && ch != '\n' {
                // Show non-printing characters using ^ and M- notation.
                // Tabs and newlines are excluded from this transformation
                // (they have their own flags).
                transformed.push_str(&format_nonprinting(ch));
            } else {
                transformed.push(ch);
            }
        }

        // --- Show line endings ---
        if options.show_ends {
            transformed.push('$');
        }

        // --- Append newline ---
        transformed.push('\n');
        output.push_str(&transformed);
    }

    output
}

/// Read content from a file path, or from stdin if path is "-".
///
/// This handles the I/O side of cat. The result is a string containing
/// the file's entire contents, ready for transformation by
/// `process_cat_content`.
pub fn read_file_or_stdin(path: &str) -> Result<String, String> {
    if path == "-" {
        // Read from standard input
        let mut buffer = String::new();
        io::stdin()
            .lock()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("cat: -: {}", e))?;
        Ok(buffer)
    } else {
        // Read from a file
        fs::read_to_string(path)
            .map_err(|e| format!("cat: {}: {}", path, e))
    }
}

// ---------------------------------------------------------------------------
// Non-printing character formatting
// ---------------------------------------------------------------------------

/// Format a non-printing character using ^ and M- notation.
///
/// This implements the same notation used by GNU `cat -v`:
///
/// ```text
///     Byte range     Notation         Example
///     ─────────────  ───────────────  ──────────────
///     0x00 – 0x1F    ^@, ^A, ..., ^_ ^@ for NUL
///     0x7F           ^?               DEL character
///     0x80 – 0x9F    M-^@, M-^A, ... High-bit control
///     0xA0 – 0xFE    M- , M-!, ...   High-bit printable
///     0xFF           M-^?             High-bit DEL
/// ```
///
/// For multi-byte UTF-8 characters (code points > 255), we simply
/// pass them through unchanged — they're printable Unicode.
fn format_nonprinting(ch: char) -> String {
    let code = ch as u32;

    if code > 255 {
        // Multi-byte Unicode character — pass through as-is.
        // GNU cat -v does the same for valid UTF-8 sequences.
        return ch.to_string();
    }

    let byte = code as u8;

    if byte < 32 {
        // Control characters (0x00-0x1F): use ^X notation
        // ^@ = 0, ^A = 1, ..., ^Z = 26, ^[ = 27, ...
        format!("^{}", (byte + 64) as char)
    } else if byte == 127 {
        // DEL character: ^?
        "^?".to_string()
    } else if byte >= 128 && byte < 160 {
        // High-bit control characters: M-^X notation
        format!("M-^{}", (byte - 128 + 64) as char)
    } else if byte >= 160 && byte < 255 {
        // High-bit printable characters: M-X notation
        format!("M-{}", (byte - 128) as char)
    } else if byte == 255 {
        // 0xFF: M-^?
        "M-^?".to_string()
    } else {
        // Regular printable ASCII (32-126): pass through
        ch.to_string()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_passthrough() {
        let opts = CatOptions::new();
        assert_eq!(
            process_cat_content("hello\nworld\n", &opts),
            "hello\nworld\n"
        );
    }

    #[test]
    fn number_all_lines() {
        let mut opts = CatOptions::new();
        opts.number = true;
        let result = process_cat_content("a\nb\nc\n", &opts);
        assert!(result.contains("     1\ta"));
        assert!(result.contains("     2\tb"));
        assert!(result.contains("     3\tc"));
    }

    #[test]
    fn number_nonblank_skips_empty() {
        let mut opts = CatOptions::new();
        opts.number_nonblank = true;
        let result = process_cat_content("a\n\nb\n", &opts);
        // Line "a" gets number 1, blank line gets no number, "b" gets 2
        assert!(result.contains("     1\ta"));
        assert!(result.contains("     2\tb"));
        // The blank line should not have a number
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], ""); // blank line, no number prefix
    }

    #[test]
    fn squeeze_blank_lines() {
        let mut opts = CatOptions::new();
        opts.squeeze_blank = true;
        let result = process_cat_content("a\n\n\n\nb\n", &opts);
        // Multiple blank lines should be squeezed to one
        assert_eq!(result, "a\n\nb\n");
    }

    #[test]
    fn show_tabs() {
        let mut opts = CatOptions::new();
        opts.show_tabs = true;
        let result = process_cat_content("hello\tworld\n", &opts);
        assert_eq!(result, "hello^Iworld\n");
    }

    #[test]
    fn show_ends() {
        let mut opts = CatOptions::new();
        opts.show_ends = true;
        let result = process_cat_content("hello\nworld\n", &opts);
        assert_eq!(result, "hello$\nworld$\n");
    }

    #[test]
    fn show_all_enables_three_flags() {
        let mut opts = CatOptions::new();
        opts.apply_show_all();
        assert!(opts.show_nonprinting);
        assert!(opts.show_ends);
        assert!(opts.show_tabs);
    }

    #[test]
    fn nonprinting_control_chars() {
        assert_eq!(format_nonprinting('\x01'), "^A");
        assert_eq!(format_nonprinting('\x00'), "^@");
        assert_eq!(format_nonprinting('\x1F'), "^_");
        assert_eq!(format_nonprinting('\x7F'), "^?");
    }

    #[test]
    fn empty_input() {
        let opts = CatOptions::new();
        assert_eq!(process_cat_content("", &opts), "");
    }

    #[test]
    fn no_trailing_newline() {
        let opts = CatOptions::new();
        // Input without trailing newline — we still process the last line
        assert_eq!(process_cat_content("hello\n", &opts), "hello\n");
    }
}
