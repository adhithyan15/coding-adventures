//! # grep — Print Lines That Match Patterns
//!
//! This module implements the business logic for the `grep` command.
//! `grep` searches text for lines matching a pattern and prints them.
//!
//! ## How Pattern Matching Works
//!
//! At its core, grep does one thing: test whether a line matches a
//! pattern. Everything else (counting, inverting, context) is built
//! on top of that primitive operation.
//!
//! ```text
//!     For each line in input:
//!         if matches(line, pattern):
//!             emit(line)
//! ```
//!
//! ## Matching Modes
//!
//! ```text
//!     Mode             Flag    How patterns are interpreted
//!     ──────────────   ─────   ─────────────────────────────────────
//!     Fixed strings    -F      Literal substring match (fastest)
//!     Basic regex      -G      BRE — default (. * ^ $ [ ] supported)
//!     Extended regex   -E      ERE — + ? { } | ( ) are special too
//! ```
//!
//! ## Implementation Note
//!
//! We implement fixed-string matching (-F) with simple `str::contains`,
//! and basic pattern matching with a minimal hand-rolled matcher that
//! supports: `.` (any char), `*` (zero or more of previous), `^` and
//! `$` (anchors), and literal characters. This avoids adding the
//! `regex` crate as a dependency.

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `grep_line` and `grep_content` behave.
///
/// ```text
///     Flag              Field            Effect
///     ──────────────    ──────────────   ──────────────────────────────
///     -i, --ignore-case ignore_case      Case-insensitive matching
///     -v, --invert      invert_match     Select non-matching lines
///     -F, --fixed       fixed_strings    Treat pattern as literal string
///     -w, --word-regexp word_regexp       Match whole words only
///     -x, --line-regexp line_regexp       Match whole lines only
///     -c, --count       count            Count matches instead of printing
///     -n, --line-number line_number       Show line numbers
/// ```
#[derive(Debug, Clone, Default)]
pub struct GrepOptions {
    /// Case-insensitive matching.
    pub ignore_case: bool,
    /// Select non-matching lines.
    pub invert_match: bool,
    /// Treat pattern as a fixed string (no regex).
    pub fixed_strings: bool,
    /// Match only whole words.
    pub word_regexp: bool,
    /// Match only whole lines.
    pub line_regexp: bool,
    /// Count matches instead of printing.
    pub count: bool,
    /// Prefix output with line numbers.
    pub line_number: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Test whether a single line matches the pattern.
///
/// This is the fundamental matching primitive. All higher-level
/// operations (searching files, counting) build on this function.
///
/// # How the Match Decision Works
///
/// ```text
///     Input: line = "Hello World", pattern = "world"
///
///     1. Apply options:
///         ignore_case? → compare "hello world" vs "world"
///         fixed_strings? → use str::contains
///         word_regexp? → check word boundaries
///         line_regexp? → check full line match
///
///     2. Determine raw match result (true/false)
///
///     3. Apply invert_match:
///         invert? → flip the result
///
///     4. Return final boolean
/// ```
pub fn grep_line(line: &str, pattern: &str, opts: &GrepOptions) -> bool {
    // --- Prepare line and pattern for comparison ---
    let (search_line, search_pattern) = if opts.ignore_case {
        (line.to_lowercase(), pattern.to_lowercase())
    } else {
        (line.to_string(), pattern.to_string())
    };

    // --- Determine if the line matches ---
    let matched = if opts.line_regexp {
        // Whole-line match: the entire line must equal the pattern
        if opts.fixed_strings {
            search_line == search_pattern
        } else {
            // For regex: anchor the pattern to match the full line
            simple_match(&format!("^{}$", search_pattern), &search_line)
        }
    } else if opts.word_regexp {
        // Word match: pattern must appear as a complete word
        word_match(&search_line, &search_pattern, opts.fixed_strings)
    } else if opts.fixed_strings {
        // Fixed string: simple substring search
        search_line.contains(&search_pattern)
    } else {
        // Basic regex matching
        simple_match(&search_pattern, &search_line)
    };

    // --- Apply inversion ---
    if opts.invert_match {
        !matched
    } else {
        matched
    }
}

/// Search content (multiple lines) and return matching lines.
///
/// This is a higher-level function that applies `grep_line` to each
/// line of the input content.
///
/// # Returns
///
/// A vector of `(line_number, line_content)` pairs for each matching line.
/// Line numbers are 1-based, matching the convention of grep output.
///
/// # Example
///
/// ```text
///     content = "apple\nbanana\ncherry"
///     pattern = "an"
///     opts = default
///     result = [(2, "banana")]
/// ```
pub fn grep_content(
    content: &str,
    pattern: &str,
    opts: &GrepOptions,
) -> Vec<(usize, String)> {
    content
        .lines()
        .enumerate()
        .filter(|(_i, line)| grep_line(line, pattern, opts))
        .map(|(i, line)| (i + 1, line.to_string()))
        .collect()
}

/// Count the number of matching lines in the content.
///
/// This is equivalent to `grep -c` — it returns just the count
/// instead of the matching lines themselves.
pub fn grep_count(content: &str, pattern: &str, opts: &GrepOptions) -> usize {
    content
        .lines()
        .filter(|line| grep_line(line, pattern, opts))
        .count()
}

// ---------------------------------------------------------------------------
// Pattern Matching Engine
// ---------------------------------------------------------------------------

/// A minimal pattern matcher supporting basic regex features.
///
/// Supported syntax:
///
/// ```text
///     .       Match any single character
///     *       Match zero or more of the preceding element
///     ^       Match start of string (only at pattern start)
///     $       Match end of string (only at pattern end)
///     \c      Escape: match character c literally
///     c       Match the literal character c
/// ```
///
/// ## How It Works
///
/// This is a recursive backtracking matcher. At each step, we try to
/// match the first element of the pattern against the current position
/// in the text. If the next pattern element is `*`, we try matching
/// zero, one, two, ... occurrences of the current element.
///
/// ```text
///     simple_match("a.*b", "aXYZb")
///     ├── pattern[0] = 'a', text[0] = 'a' → match
///     ├── pattern[1] = '.', pattern[2] = '*' → star match
///     │   ├── try 0 repeats: match("b", "XYZb") → no
///     │   ├── try 1 repeat:  match("b", "YZb")  → no
///     │   ├── try 2 repeats: match("b", "Zb")   → no
///     │   └── try 3 repeats: match("b", "b")    → yes!
///     └── result: true
/// ```
fn simple_match(pattern: &str, text: &str) -> bool {
    let pat_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    // --- Handle ^ anchor ---
    if pat_chars.first() == Some(&'^') {
        return match_here(&pat_chars[1..], &text_chars);
    }

    // --- Try matching at every position (unanchored) ---
    // grep semantics: the pattern can match anywhere in the line
    for i in 0..=text_chars.len() {
        if match_here(&pat_chars, &text_chars[i..]) {
            return true;
        }
    }

    false
}

/// Match pattern at the current position in text.
///
/// This is the recursive core of the matcher. It handles:
/// - Empty pattern (always matches)
/// - `$` at end of pattern (must be at end of text)
/// - `c*` (star: zero or more of c)
/// - `.` (any character)
/// - `\c` (escaped literal)
/// - Literal character
fn match_here(pattern: &[char], text: &[char]) -> bool {
    // --- Base case: empty pattern matches anything ---
    if pattern.is_empty() {
        return true;
    }

    // --- $ anchor: must be at end of text ---
    if pattern.len() == 1 && pattern[0] == '$' {
        return text.is_empty();
    }

    // --- Handle escaped characters ---
    if pattern[0] == '\\' && pattern.len() >= 2 {
        // The next character is a literal
        let literal = pattern[1];
        if pattern.len() >= 3 && pattern[2] == '*' {
            return match_star(literal, true, &pattern[3..], text);
        }
        if !text.is_empty() && text[0] == literal {
            return match_here(&pattern[2..], &text[1..]);
        }
        return false;
    }

    // --- Star: zero or more of preceding element ---
    if pattern.len() >= 2 && pattern[1] == '*' {
        return match_star(pattern[0], false, &pattern[2..], text);
    }

    // --- Dot: any single character ---
    if !text.is_empty() && (pattern[0] == '.' || pattern[0] == text[0]) {
        return match_here(&pattern[1..], &text[1..]);
    }

    false
}

/// Match `c*` (zero or more of character c) followed by the rest of
/// the pattern.
///
/// We use a "greedy then backtrack" approach: try consuming as many
/// characters as possible, then back off one at a time.
///
/// Actually, for simplicity and correctness, we use a "try each count"
/// approach starting from zero:
///
/// ```text
///     match_star('.', rest_pattern, "XYZ"):
///         try 0: match(rest, "XYZ")
///         try 1: match(rest, "YZ")
///         try 2: match(rest, "Z")
///         try 3: match(rest, "")
/// ```
fn match_star(c: char, is_literal: bool, rest: &[char], text: &[char]) -> bool {
    let mut i = 0;
    loop {
        // Try matching the rest of the pattern at position i
        if match_here(rest, &text[i..]) {
            return true;
        }
        // Can we consume one more character?
        if i >= text.len() {
            return false;
        }
        if is_literal {
            if text[i] != c {
                return false;
            }
        } else if c != '.' && text[i] != c {
            return false;
        }
        i += 1;
    }
}

/// Check if the pattern appears as a whole word in the text.
///
/// A "word" is bounded by non-alphanumeric characters (or start/end
/// of string). This implements `grep -w` semantics.
///
/// ```text
///     word_match("cat catalog", "cat") → true  (matches "cat" at start)
///     word_match("catalog",     "cat") → false (not a whole word)
/// ```
fn word_match(text: &str, pattern: &str, fixed: bool) -> bool {
    if fixed {
        // For fixed strings, find all occurrences and check boundaries
        let pat_len = pattern.len();
        let text_bytes = text.as_bytes();

        let mut start = 0;
        while start + pat_len <= text.len() {
            if let Some(pos) = text[start..].find(pattern) {
                let abs_pos = start + pos;
                let before_ok = abs_pos == 0
                    || !is_word_char(text_bytes[abs_pos - 1]);
                let after_ok = abs_pos + pat_len >= text.len()
                    || !is_word_char(text_bytes[abs_pos + pat_len]);
                if before_ok && after_ok {
                    return true;
                }
                start = abs_pos + 1;
            } else {
                break;
            }
        }
        false
    } else {
        // For regex patterns, wrap in word boundary simulation.
        // We find matches using simple_match and check boundaries.
        // Simplified: split text into words and check each
        let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric() && c != '_').collect();
        words.iter().any(|word| simple_match(&pattern, word))
    }
}

/// Check if a byte is a "word character" (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Fixed string matching ---

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

    // --- Basic regex matching ---

    #[test]
    fn regex_dot_matches_any() {
        let opts = GrepOptions::default();
        assert!(grep_line("cat", "c.t", &opts));
        assert!(grep_line("cot", "c.t", &opts));
        assert!(!grep_line("ct", "c.t", &opts));
    }

    #[test]
    fn regex_star_matches_zero_or_more() {
        let opts = GrepOptions::default();
        assert!(grep_line("ct", "ca*t", &opts));
        assert!(grep_line("cat", "ca*t", &opts));
        assert!(grep_line("caat", "ca*t", &opts));
    }

    #[test]
    fn regex_caret_anchors_start() {
        let opts = GrepOptions::default();
        assert!(grep_line("hello world", "^hello", &opts));
        assert!(!grep_line("say hello", "^hello", &opts));
    }

    #[test]
    fn regex_dollar_anchors_end() {
        let opts = GrepOptions::default();
        assert!(grep_line("hello world", "world$", &opts));
        assert!(!grep_line("world hello", "world$", &opts));
    }

    #[test]
    fn regex_dot_star_matches_anything() {
        let opts = GrepOptions::default();
        assert!(grep_line("anything at all", "any.*all", &opts));
        assert!(grep_line("aXb", "a.*b", &opts));
    }

    // --- Invert match ---

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

    // --- Line regexp ---

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

    // --- Word regexp ---

    #[test]
    fn word_regexp_fixed() {
        let opts = GrepOptions {
            word_regexp: true,
            fixed_strings: true,
            ..Default::default()
        };
        assert!(grep_line("the cat sat", "cat", &opts));
        assert!(!grep_line("concatenate", "cat", &opts));
    }

    // --- grep_content ---

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

    // --- grep_count ---

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

    // --- Edge cases ---

    #[test]
    fn empty_pattern_matches_everything() {
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert!(grep_line("anything", "", &opts));
    }

    #[test]
    fn empty_line_matches_empty_pattern() {
        let opts = GrepOptions { fixed_strings: true, ..Default::default() };
        assert!(grep_line("", "", &opts));
    }

    #[test]
    fn escaped_dot_matches_literal() {
        let opts = GrepOptions::default();
        assert!(grep_line("file.txt", "file\\.txt", &opts));
        assert!(!grep_line("fileXtxt", "file\\.txt", &opts));
    }

    #[test]
    fn simple_match_unanchored() {
        // Pattern can match anywhere in the line (grep semantics)
        let opts = GrepOptions::default();
        assert!(grep_line("the cat sat", "cat", &opts));
        assert!(grep_line("catalog", "cat", &opts));
    }
}
