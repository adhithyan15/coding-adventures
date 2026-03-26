//! # paste — Merge Lines of Files
//!
//! This module implements the business logic for the `paste` command.
//! `paste` merges corresponding lines from multiple inputs, joining
//! them with a delimiter (TAB by default).
//!
//! ## Two Modes of Operation
//!
//! ```text
//!     Mode       Flag    Description
//!     ─────────  ─────   ──────────────────────────────────────
//!     Parallel   (none)  Merge line N from each file side by side
//!     Serial     -s      Paste each file's lines into one output line
//! ```
//!
//! ## Parallel Mode (Default)
//!
//! ```text
//!     File1:    File2:    Output:
//!     a         1         a\t1
//!     b         2         b\t2
//!     c         3         c\t3
//! ```
//!
//! If files have different lengths, missing fields are empty:
//!
//! ```text
//!     File1:    File2:    Output:
//!     a         1         a\t1
//!     b                   b\t
//! ```
//!
//! ## Serial Mode (-s)
//!
//! ```text
//!     File1 (lines: a, b, c) → "a\tb\tc"
//!     File2 (lines: 1, 2, 3) → "1\t2\t3"
//! ```
//!
//! ## Delimiter Cycling
//!
//! The -d flag accepts a list of delimiters that cycle for each join:
//!
//! ```text
//!     paste -d ':,' file1 file2 file3
//!     → line1_f1 : line1_f2 , line1_f3
//!     → line2_f1 : line2_f2 , line2_f3
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Merge lines from multiple inputs in parallel.
///
/// Each element in `inputs` is a Vec of lines (one per file).
/// Lines from each file are joined with delimiters from `delimiters`,
/// cycling through the delimiter list.
///
/// # Parameters
///
/// - `inputs`: a slice of Vecs, one Vec per input file
/// - `delimiters`: the delimiter string (each char is used in turn)
/// - `serial`: if true, paste each file's lines into one output line
///
/// # Returns
///
/// A Vec of output lines (without trailing newlines).
///
/// # Example
///
/// ```text
///     inputs = [["a", "b"], ["1", "2"]]
///     delimiters = "\t"
///     serial = false
///     → ["a\t1", "b\t2"]
/// ```
pub fn paste_lines(inputs: &[Vec<String>], delimiters: &str, serial: bool) -> Vec<String> {
    // Default delimiter is TAB if none specified
    let delims: Vec<char> = if delimiters.is_empty() {
        vec!['\t']
    } else {
        parse_delimiters(delimiters)
    };

    if serial {
        paste_serial(inputs, &delims)
    } else {
        paste_parallel(inputs, &delims)
    }
}

/// Parse delimiter string, handling escape sequences.
///
/// ```text
///     "\\t" → ['\t']
///     "\\n" → ['\n']
///     ":"   → [':']
///     ":,"  → [':', ',']
/// ```
fn parse_delimiters(s: &str) -> Vec<char> {
    let mut result = Vec::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('t') => result.push('\t'),
                Some('n') => result.push('\n'),
                Some('\\') => result.push('\\'),
                Some('0') => result.push('\0'),
                Some(other) => result.push(other),
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }

    if result.is_empty() {
        vec!['\t']
    } else {
        result
    }
}

/// Paste files in parallel — merge corresponding lines.
///
/// ```text
///     Line 1 from each file → joined by delims[0], delims[1], ...
///     Line 2 from each file → joined by delims[0], delims[1], ...
/// ```
fn paste_parallel(inputs: &[Vec<String>], delims: &[char]) -> Vec<String> {
    if inputs.is_empty() {
        return Vec::new();
    }

    // Find the maximum number of lines across all inputs
    let max_lines = inputs.iter().map(|f| f.len()).max().unwrap_or(0);
    let mut output = Vec::with_capacity(max_lines);

    for line_idx in 0..max_lines {
        let mut parts = Vec::new();
        for (file_idx, file) in inputs.iter().enumerate() {
            let field = if line_idx < file.len() {
                file[line_idx].as_str()
            } else {
                ""
            };

            if file_idx == 0 {
                parts.push(field.to_string());
            } else {
                // Use cycling delimiter
                let delim_idx = (file_idx - 1) % delims.len();
                let d = delims[delim_idx];
                parts.push(format!("{}{}", d, field));
            }
        }
        output.push(parts.join(""));
    }

    output
}

/// Paste files serially — each file becomes one output line.
///
/// ```text
///     File1 lines: [a, b, c] → "a\tb\tc"
///     File2 lines: [1, 2, 3] → "1\t2\t3"
/// ```
fn paste_serial(inputs: &[Vec<String>], delims: &[char]) -> Vec<String> {
    let mut output = Vec::new();

    for file in inputs {
        if file.is_empty() {
            output.push(String::new());
            continue;
        }

        let mut line = file[0].clone();
        for (i, field) in file.iter().enumerate().skip(1) {
            let delim_idx = (i - 1) % delims.len();
            line.push(delims[delim_idx]);
            line.push_str(field);
        }
        output.push(line);
    }

    output
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_two_files() {
        let inputs = vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
        ];
        let result = paste_lines(&inputs, "\t", false);
        assert_eq!(result, vec!["a\t1", "b\t2"]);
    }

    #[test]
    fn parallel_unequal_lengths() {
        let inputs = vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["1".into()],
        ];
        let result = paste_lines(&inputs, "\t", false);
        assert_eq!(result, vec!["a\t1", "b\t", "c\t"]);
    }

    #[test]
    fn serial_mode() {
        let inputs = vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["1".into(), "2".into(), "3".into()],
        ];
        let result = paste_lines(&inputs, "\t", true);
        assert_eq!(result, vec!["a\tb\tc", "1\t2\t3"]);
    }

    #[test]
    fn custom_delimiter() {
        let inputs = vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
        ];
        let result = paste_lines(&inputs, ":", false);
        assert_eq!(result, vec!["a:1", "b:2"]);
    }

    #[test]
    fn cycling_delimiters() {
        let inputs = vec![
            vec!["a".into()],
            vec!["b".into()],
            vec!["c".into()],
        ];
        let result = paste_lines(&inputs, ":,", false);
        assert_eq!(result, vec!["a:b,c"]);
    }

    #[test]
    fn empty_inputs() {
        let inputs: Vec<Vec<String>> = vec![];
        let result = paste_lines(&inputs, "\t", false);
        assert!(result.is_empty());
    }

    #[test]
    fn serial_single_line_files() {
        let inputs = vec![
            vec!["hello".into()],
            vec!["world".into()],
        ];
        let result = paste_lines(&inputs, "\t", true);
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn parse_delimiter_escapes() {
        assert_eq!(parse_delimiters("\\t"), vec!['\t']);
        assert_eq!(parse_delimiters("\\n"), vec!['\n']);
        assert_eq!(parse_delimiters(":,"), vec![':', ',']);
    }

    #[test]
    fn empty_delimiter_defaults_to_tab() {
        let inputs = vec![
            vec!["a".into()],
            vec!["b".into()],
        ];
        let result = paste_lines(&inputs, "", false);
        assert_eq!(result, vec!["a\tb"]);
    }

    #[test]
    fn serial_empty_file() {
        let inputs = vec![
            vec![],
            vec!["1".into(), "2".into()],
        ];
        let result = paste_lines(&inputs, "\t", true);
        assert_eq!(result, vec!["", "1\t2"]);
    }

    #[test]
    fn three_files_parallel() {
        let inputs = vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
            vec!["x".into(), "y".into()],
        ];
        let result = paste_lines(&inputs, "\t", false);
        assert_eq!(result, vec!["a\t1\tx", "b\t2\ty"]);
    }
}
