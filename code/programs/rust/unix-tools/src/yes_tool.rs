//! # yes — Output a String Repeatedly
//!
//! This module implements the business logic for the `yes` command.
//! The `yes` utility repeatedly outputs a line with the string "y"
//! (or a user-specified string) until killed.
//!
//! ## Usage
//!
//! ```text
//!     yes              Output "y" forever
//!     yes hello        Output "hello" forever
//!     yes hello world  Output "hello world" forever
//! ```
//!
//! ## How It Works
//!
//! When called with no arguments, `yes` outputs "y" on each line.
//! When called with arguments, it joins them with spaces and outputs
//! that string on each line. The output continues until the process
//! is terminated (usually by a pipe closing or Ctrl-C).
//!
//! ## Common Uses
//!
//! ```text
//!     # Auto-answer "yes" to interactive prompts
//!     yes | rm -i *.tmp
//!
//!     # Auto-answer with a custom string
//!     yes "I agree" | some_license_tool
//!
//!     # Generate test data (first 1000 lines)
//!     yes "test line" | head -n 1000 > test_data.txt
//! ```
//!
//! ## Design
//!
//! The `yes_output` function takes a `max_lines` parameter so that
//! the business logic is testable without running an infinite loop.
//! The real `yes` binary would call this with `usize::MAX` (or loop
//! until a broken pipe signal).

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate repeated output lines of the given string.
///
/// This is the core business logic for `yes`. It produces up to
/// `max_lines` lines, each containing `text` followed by a newline.
///
/// # Parameters
///
/// - `text`: The string to repeat on each line. If empty, defaults to "y".
/// - `max_lines`: Maximum number of lines to produce. Use this to make
///   the function testable without infinite loops.
///
/// # Returns
///
/// A `String` containing the repeated lines.
///
/// # Example
///
/// ```text
///     yes_output("y", 3)    => "y\ny\ny\n"
///     yes_output("hi", 2)   => "hi\nhi\n"
///     yes_output("", 2)     => "y\ny\n"
/// ```
pub fn yes_output(text: &str, max_lines: usize) -> String {
    // --- Handle the default case ---
    // If no string is provided (empty), POSIX says to use "y".
    let output_text = if text.is_empty() { "y" } else { text };

    // --- Build the output ---
    // Pre-allocate for efficiency. Each line is the text plus a newline.
    let line_len = output_text.len() + 1; // +1 for '\n'
    let mut output = String::with_capacity(line_len * max_lines);

    for _ in 0..max_lines {
        output.push_str(output_text);
        output.push('\n');
    }

    output
}

/// Join arguments with spaces to form the yes output string.
///
/// When `yes` receives multiple arguments, they are joined with
/// spaces — just like `echo`. This function handles that joining.
///
/// # Example
///
/// ```text
///     join_args(&["hello".into(), "world".into()])  => "hello world"
///     join_args(&[])                                 => ""
/// ```
pub fn join_args(args: &[String]) -> String {
    args.join(" ")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_is_y() {
        let result = yes_output("", 3);
        assert_eq!(result, "y\ny\ny\n");
    }

    #[test]
    fn custom_string() {
        let result = yes_output("hello", 2);
        assert_eq!(result, "hello\nhello\n");
    }

    #[test]
    fn zero_lines() {
        let result = yes_output("y", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn single_line() {
        let result = yes_output("test", 1);
        assert_eq!(result, "test\n");
    }

    #[test]
    fn multi_word_string() {
        let result = yes_output("hello world", 2);
        assert_eq!(result, "hello world\nhello world\n");
    }

    #[test]
    fn join_multiple_args() {
        let args: Vec<String> = vec!["hello".into(), "world".into()];
        assert_eq!(join_args(&args), "hello world");
    }

    #[test]
    fn join_single_arg() {
        let args: Vec<String> = vec!["hello".into()];
        assert_eq!(join_args(&args), "hello");
    }

    #[test]
    fn join_empty_args() {
        let args: Vec<String> = vec![];
        assert_eq!(join_args(&args), "");
    }

    #[test]
    fn large_output() {
        let result = yes_output("y", 100);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 100);
        assert!(lines.iter().all(|l| *l == "y"));
    }
}
