//! # uniq — Report or Omit Repeated Lines
//!
//! This module implements the business logic for the `uniq` command.
//! uniq filters ADJACENT duplicate lines. To find all duplicates in
//! a file, sort it first: `sort file | uniq`.
//!
//! ## Key Behavior
//!
//! ```text
//!     Input:          Output (default):
//!     apple           apple
//!     apple           banana
//!     banana          apple
//!     apple
//! ```
//!
//! Notice "apple" appears twice in the output because the two groups
//! are separated by "banana."
//!
//! ## Modes
//!
//! ```text
//!     uniq            Remove adjacent duplicates (keep first of each group)
//!     uniq -c         Prefix each line with its count
//!     uniq -d         Show only duplicated lines
//!     uniq -u         Show only unique (non-duplicated) lines
//!     uniq -i         Ignore case when comparing
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A group of adjacent identical lines.
#[derive(Debug, Clone)]
pub struct UniqGroup {
    /// The original line text (first occurrence).
    pub line: String,
    /// How many consecutive times this line appeared.
    pub count: usize,
}

/// Process content through the uniq algorithm.
///
/// Groups adjacent identical lines and filters/formats them according
/// to the provided options.
///
/// # Parameters
/// - `content`: the input text
/// - `show_count`: prefix each line with its occurrence count (-c)
/// - `repeated_only`: only output lines that appeared more than once (-d)
/// - `unique_only`: only output lines that appeared exactly once (-u)
/// - `ignore_case`: compare lines case-insensitively (-i)
/// - `skip_fields`: skip the first N whitespace-delimited fields (-f)
/// - `skip_chars`: skip the first N characters after field skipping (-s)
/// - `check_chars`: compare only the first N characters (-w)
pub fn process_uniq(
    content: &str,
    show_count: bool,
    repeated_only: bool,
    unique_only: bool,
    ignore_case: bool,
    skip_fields: usize,
    skip_chars: usize,
    check_chars: usize,
) -> String {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    // Group adjacent identical lines.
    let mut groups: Vec<UniqGroup> = Vec::new();
    let mut current = UniqGroup {
        line: lines[0].to_string(),
        count: 1,
    };

    for line in &lines[1..] {
        let key1 = compare_key(&current.line, skip_fields, skip_chars, check_chars, ignore_case);
        let key2 = compare_key(line, skip_fields, skip_chars, check_chars, ignore_case);

        if key1 == key2 {
            current.count += 1;
        } else {
            groups.push(current);
            current = UniqGroup {
                line: line.to_string(),
                count: 1,
            };
        }
    }
    groups.push(current);

    // Filter and format output.
    let mut result = String::new();
    for group in &groups {
        if repeated_only && group.count < 2 {
            continue;
        }
        if unique_only && group.count > 1 {
            continue;
        }

        if show_count {
            result.push_str(&format!("{:7} {}\n", group.count, group.line));
        } else {
            result.push_str(&group.line);
            result.push('\n');
        }
    }

    result
}

/// Extract the comparison key from a line, applying skip and check options.
fn compare_key(
    line: &str,
    skip_fields: usize,
    skip_chars: usize,
    check_chars: usize,
    ignore_case: bool,
) -> String {
    let mut remaining = line;

    // Skip fields.
    for _ in 0..skip_fields {
        remaining = remaining.trim_start_matches(|c: char| c == ' ' || c == '\t');
        match remaining.find(|c: char| c == ' ' || c == '\t') {
            Some(idx) => remaining = &remaining[idx..],
            None => {
                remaining = "";
                break;
            }
        }
    }

    // Skip characters.
    let chars: Vec<char> = remaining.chars().collect();
    let start = skip_chars.min(chars.len());
    let end = if check_chars > 0 {
        (start + check_chars).min(chars.len())
    } else {
        chars.len()
    };

    let result: String = chars[start..end].iter().collect();

    if ignore_case {
        result.to_lowercase()
    } else {
        result
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_dedup() {
        assert_eq!(
            process_uniq("a\na\nb\n", false, false, false, false, 0, 0, 0),
            "a\nb\n"
        );
    }

    #[test]
    fn count_mode() {
        let result = process_uniq("a\na\nb\n", true, false, false, false, 0, 0, 0);
        assert!(result.contains("2 a"));
        assert!(result.contains("1 b"));
    }

    #[test]
    fn repeated_only() {
        assert_eq!(
            process_uniq("a\na\nb\n", false, true, false, false, 0, 0, 0),
            "a\n"
        );
    }

    #[test]
    fn unique_only() {
        assert_eq!(
            process_uniq("a\na\nb\n", false, false, true, false, 0, 0, 0),
            "b\n"
        );
    }

    #[test]
    fn ignore_case() {
        assert_eq!(
            process_uniq("Apple\napple\nBANANA\n", false, false, false, true, 0, 0, 0),
            "Apple\nBANANA\n"
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(
            process_uniq("", false, false, false, false, 0, 0, 0),
            ""
        );
    }

    #[test]
    fn no_adjacent_dupes() {
        assert_eq!(
            process_uniq("a\nb\nc\n", false, false, false, false, 0, 0, 0),
            "a\nb\nc\n"
        );
    }

    #[test]
    fn compare_key_skip_fields() {
        let key = compare_key("field1 field2 data", 2, 0, 0, false);
        assert_eq!(key, "data");
    }

    #[test]
    fn compare_key_ignore_case() {
        let key = compare_key("HELLO", 0, 0, 0, true);
        assert_eq!(key, "hello");
    }
}
