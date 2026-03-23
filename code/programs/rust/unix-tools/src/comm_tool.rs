//! # comm — Compare Two Sorted Files Line by Line
//!
//! This module implements the business logic for the `comm` command.
//! `comm` reads two sorted files and produces three columns of output:
//!
//! ```text
//!     Column 1: Lines unique to FILE1
//!     Column 2: Lines unique to FILE2
//!     Column 3: Lines common to both files
//! ```
//!
//! ## Visual Example
//!
//! ```text
//!     FILE1:    FILE2:    Output:
//!     apple     banana    apple            ← only in FILE1
//!     banana    cherry            banana   ← only in FILE2 (wait, banana is in both!)
//!     cherry    date              banana   ← common (column 3)
//!     fig       fig                       cherry  ← hmm, let's trace carefully...
//! ```
//!
//! Actually, let's trace the merge-like algorithm step by step:
//!
//! ```text
//!     FILE1:    FILE2:    Step:
//!     apple     banana    "apple" < "banana" → col1: "apple"
//!     banana    cherry    "banana" == "banana" → col3: "\t\tbanana"
//!     cherry    date      "cherry" == "cherry" → col3: "\t\tcherry"
//!     fig       fig       "date" < "fig" → col2: "\tdate"
//!                         "fig" == "fig" → col3: "\t\tfig"
//! ```
//!
//! ## Suppressing Columns
//!
//! The `-1`, `-2`, and `-3` flags suppress individual columns.
//! When a column is suppressed, its tab prefix is also removed,
//! so the remaining columns shift left.
//!
//! ## Requirement: Input Must Be Sorted
//!
//! `comm` assumes both files are sorted. If they aren't, the output
//! is undefined (and likely wrong). The `--check-order` flag makes
//! this an explicit error.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compare two sorted lists of lines and produce comm-style output.
///
/// # Parameters
///
/// - `lines1`: lines from FILE1, must be sorted
/// - `lines2`: lines from FILE2, must be sorted
/// - `suppress`: which columns to suppress: `[col1, col2, col3]`
/// - `output_delimiter`: separator between columns (default: TAB)
///
/// # Algorithm
///
/// This uses a merge-join algorithm, similar to merge sort's merge step.
/// We maintain two pointers (one into each file) and advance them
/// based on comparison:
///
/// ```text
///     if line1 < line2:  emit col1, advance ptr1
///     if line1 > line2:  emit col2, advance ptr2
///     if line1 == line2: emit col3, advance both
/// ```
///
/// This is O(n + m) where n and m are the lengths of the two files.
pub fn compare_sorted(
    lines1: &[String],
    lines2: &[String],
    suppress: [bool; 3],
    output_delimiter: &str,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    // The delimiter used for indentation. By default, TAB is used
    // to separate columns. Suppressed columns don't get a tab prefix.
    let sep = if output_delimiter.is_empty() { "\t" } else { output_delimiter };

    while i < lines1.len() && j < lines2.len() {
        match lines1[i].cmp(&lines2[j]) {
            std::cmp::Ordering::Less => {
                // Line is unique to FILE1 → column 1
                if !suppress[0] {
                    result.push(lines1[i].clone());
                }
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                // Line is unique to FILE2 → column 2
                if !suppress[1] {
                    let prefix = if suppress[0] { "" } else { sep };
                    result.push(format!("{}{}", prefix, lines2[j]));
                }
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                // Line is common to both → column 3
                if !suppress[2] {
                    let mut prefix = String::new();
                    if !suppress[0] { prefix.push_str(sep); }
                    if !suppress[1] { prefix.push_str(sep); }
                    result.push(format!("{}{}", prefix, lines1[i]));
                }
                i += 1;
                j += 1;
            }
        }
    }

    // --- Remaining lines from FILE1 ---
    while i < lines1.len() {
        if !suppress[0] {
            result.push(lines1[i].clone());
        }
        i += 1;
    }

    // --- Remaining lines from FILE2 ---
    while j < lines2.len() {
        if !suppress[1] {
            let prefix = if suppress[0] { "" } else { sep };
            result.push(format!("{}{}", prefix, lines2[j]));
        }
        j += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn basic_three_columns() {
        let f1 = s(&["apple", "cherry", "fig"]);
        let f2 = s(&["banana", "cherry", "date"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec![
            "apple",
            "\tbanana",
            "\t\tcherry",
            "\tdate",
            "fig",
        ]);
    }

    #[test]
    fn suppress_col1() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [true, false, false], "\t");
        // Column 1 suppressed, so col2 shifts left
        assert_eq!(result, vec!["\tb", "c"]);
    }

    #[test]
    fn suppress_col2() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, true, false], "\t");
        assert_eq!(result, vec!["a", "\tb"]);
    }

    #[test]
    fn suppress_col3() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, false, true], "\t");
        assert_eq!(result, vec!["a", "\tc"]);
    }

    #[test]
    fn suppress_col1_and_col2() {
        let f1 = s(&["a", "b", "c"]);
        let f2 = s(&["b", "d"]);
        let result = compare_sorted(&f1, &f2, [true, true, false], "\t");
        // Only common lines remain
        assert_eq!(result, vec!["b"]);
    }

    #[test]
    fn empty_file1() {
        let f1: Vec<String> = Vec::new();
        let f2 = s(&["a", "b"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["\ta", "\tb"]);
    }

    #[test]
    fn empty_file2() {
        let f1 = s(&["a", "b"]);
        let f2: Vec<String> = Vec::new();
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn both_empty() {
        let f1: Vec<String> = Vec::new();
        let f2: Vec<String> = Vec::new();
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert!(result.is_empty());
    }

    #[test]
    fn identical_files() {
        let f1 = s(&["a", "b", "c"]);
        let f2 = s(&["a", "b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["\t\ta", "\t\tb", "\t\tc"]);
    }

    #[test]
    fn no_common_lines() {
        let f1 = s(&["a", "c"]);
        let f2 = s(&["b", "d"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "\t");
        assert_eq!(result, vec!["a", "\tb", "c", "\td"]);
    }

    #[test]
    fn custom_output_delimiter() {
        let f1 = s(&["a", "b"]);
        let f2 = s(&["b", "c"]);
        let result = compare_sorted(&f1, &f2, [false, false, false], "|");
        assert_eq!(result, vec!["a", "||b", "|c"]);
    }
}
