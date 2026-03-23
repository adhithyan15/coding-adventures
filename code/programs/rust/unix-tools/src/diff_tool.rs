//! # diff — Compare Files Line by Line
//!
//! This module implements the business logic for the `diff` command.
//! `diff` compares two files line by line and outputs the differences
//! between them.
//!
//! ## Output Formats
//!
//! ```text
//!     Format       Flag    Description
//!     ───────────  ─────   ──────────────────────────────────────────
//!     Normal       (none)  Traditional diff output with <, >, ---
//!     Unified      -u      Shows context with +/- markers (like git)
//!     Context      -c      Shows context with ! markers
//! ```
//!
//! ## How Diffing Works
//!
//! At its core, `diff` solves the **Longest Common Subsequence (LCS)**
//! problem. Given two sequences of lines, we want to find the longest
//! subsequence that appears in both. Everything not in the LCS is a
//! change — either an addition, a deletion, or a modification.
//!
//! ### The LCS Algorithm
//!
//! We use dynamic programming to build an LCS table:
//!
//! ```text
//!     File A: ["a", "b", "c", "d"]
//!     File B: ["a", "c", "d", "e"]
//!
//!     LCS Table (dp[i][j] = LCS length of A[0..i] and B[0..j]):
//!
//!            ""  "a"  "c"  "d"  "e"
//!     ""      0    0    0    0    0
//!     "a"     0    1    1    1    1
//!     "b"     0    1    1    1    1
//!     "c"     0    1    2    2    2
//!     "d"     0    1    2    3    3
//!
//!     LCS = ["a", "c", "d"]
//!     Diff: "b" deleted from A, "e" added in B
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag                 Field            Effect
//!     ──────────────────   ──────────────   ─────────────────────────────────
//!     -i, --ignore-case    ignore_case      Case-insensitive comparison
//!     -b, --ignore-space   ignore_space     Ignore changes in whitespace amount
//!     -w, --ignore-all-sp  ignore_all_ws    Ignore all whitespace
//!     -B, --ignore-blank   ignore_blank     Ignore blank line changes
//!     -q, --brief          brief            Only report whether files differ
//!     -r, --recursive      recursive        Recursively compare directories
//!     -u, --unified        unified          Unified output format
//!     -c, --context        context          Context output format
//! ```

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `diff_files` behaves.
///
/// Each field maps to a command-line flag from the diff spec.
#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Case-insensitive line comparison (-i).
    pub ignore_case: bool,
    /// Ignore changes in the amount of whitespace (-b).
    pub ignore_space_change: bool,
    /// Ignore all whitespace (-w).
    pub ignore_all_space: bool,
    /// Ignore changes whose lines are all blank (-B).
    pub ignore_blank_lines: bool,
    /// Report only whether files differ, not the details (-q).
    pub brief: bool,
    /// Recursively compare directories (-r).
    pub recursive: bool,
    /// Number of context lines for unified/context format.
    /// 0 means use normal format.
    pub unified_context: Option<usize>,
    /// Number of context lines for context format.
    pub context_lines: Option<usize>,
}

// ---------------------------------------------------------------------------
// Edit Script
// ---------------------------------------------------------------------------

/// Represents a single difference between two files.
///
/// Each edit describes what happened to a range of lines:
///
/// ```text
///     Edit        Meaning
///     ─────────   ─────────────────────────────────────────
///     Equal       Lines are the same in both files
///     Delete      Lines exist in file A but not in file B
///     Insert      Lines exist in file B but not in file A
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Lines present in both files (unchanged).
    Equal(String),
    /// Line deleted from file A (not in file B).
    Delete(String),
    /// Line inserted in file B (not in file A).
    Insert(String),
}

// ---------------------------------------------------------------------------
// Line Normalization
// ---------------------------------------------------------------------------

/// Normalize a line according to the comparison options.
///
/// This is the key to implementing -i, -b, and -w flags. Instead of
/// modifying the diff algorithm itself, we normalize lines before
/// comparing them. This keeps the algorithm simple and the flags
/// composable.
///
/// ```text
///     Flag    Input              Normalized
///     ─────   ─────────────────  ────────────────
///     -i      "Hello World"     "hello world"
///     -b      "a   b  c"        "a b c"
///     -w      "a   b  c"        "abc"
/// ```
fn normalize_line(line: &str, opts: &DiffOptions) -> String {
    let mut result = line.to_string();

    if opts.ignore_case {
        result = result.to_lowercase();
    }

    if opts.ignore_all_space {
        // Remove all whitespace characters entirely.
        result = result.chars().filter(|c| !c.is_whitespace()).collect();
    } else if opts.ignore_space_change {
        // Collapse runs of whitespace into a single space and trim.
        let mut prev_was_space = false;
        let collapsed: String = result
            .chars()
            .filter_map(|c| {
                if c.is_whitespace() {
                    if prev_was_space {
                        None
                    } else {
                        prev_was_space = true;
                        Some(' ')
                    }
                } else {
                    prev_was_space = false;
                    Some(c)
                }
            })
            .collect();
        result = collapsed.trim().to_string();
    }

    result
}

// ---------------------------------------------------------------------------
// LCS-based Diff
// ---------------------------------------------------------------------------

/// Compute the edit script between two slices of lines using LCS.
///
/// The algorithm works in two phases:
///
/// 1. **Build the LCS table** — A 2D matrix where dp[i][j] holds the
///    length of the longest common subsequence of lines_a[0..i] and
///    lines_b[0..j].
///
/// 2. **Backtrack to produce edits** — Starting from dp[m][n], we walk
///    backwards:
///    - If lines match: it's an Equal edit, move diagonally.
///    - If dp[i-1][j] >= dp[i][j-1]: it's a Delete, move up.
///    - Otherwise: it's an Insert, move left.
///
/// ```text
///     Backtracking decision tree:
///
///         lines_a[i-1] == lines_b[j-1]?
///         ├── Yes → Equal (move diagonal ↖)
///         └── No
///             ├── dp[i-1][j] >= dp[i][j-1] → Delete (move up ↑)
///             └── Otherwise → Insert (move left ←)
/// ```
pub fn compute_diff(lines_a: &[&str], lines_b: &[&str], opts: &DiffOptions) -> Vec<Edit> {
    let m = lines_a.len();
    let n = lines_b.len();

    // --- Phase 1: Build the LCS table ---
    // dp[i][j] = length of LCS of lines_a[0..i] and lines_b[0..j]
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            let a_norm = normalize_line(lines_a[i - 1], opts);
            let b_norm = normalize_line(lines_b[j - 1], opts);
            if a_norm == b_norm {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // --- Phase 2: Backtrack to produce edits ---
    let mut edits = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let a_norm = normalize_line(lines_a[i - 1], opts);
            let b_norm = normalize_line(lines_b[j - 1], opts);
            if a_norm == b_norm {
                edits.push(Edit::Equal(lines_a[i - 1].to_string()));
                i -= 1;
                j -= 1;
                continue;
            }
        }

        if i > 0 && (j == 0 || dp[i - 1][j] >= dp[i][j - 1]) {
            edits.push(Edit::Delete(lines_a[i - 1].to_string()));
            i -= 1;
        } else if j > 0 {
            edits.push(Edit::Insert(lines_b[j - 1].to_string()));
            j -= 1;
        }
    }

    // Edits are produced in reverse order; flip them.
    edits.reverse();
    edits
}

// ---------------------------------------------------------------------------
// Output Formatting
// ---------------------------------------------------------------------------

/// Format edits in "normal" diff output.
///
/// Normal diff output looks like:
///
/// ```text
///     2d1
///     < deleted line
///     ---
///     3a3
///     > inserted line
/// ```
///
/// The header format is: `{a_range}{op}{b_range}` where op is:
/// - `a` for append (lines added in B)
/// - `d` for delete (lines removed from A)
/// - `c` for change (lines changed between A and B)
pub fn format_normal(edits: &[Edit]) -> String {
    let mut output = String::new();
    let mut a_line = 1usize;
    let mut b_line = 1usize;
    let mut idx = 0;

    while idx < edits.len() {
        match &edits[idx] {
            Edit::Equal(_) => {
                a_line += 1;
                b_line += 1;
                idx += 1;
            }
            Edit::Delete(_) | Edit::Insert(_) => {
                // Collect a contiguous block of changes.
                let a_start = a_line;
                let b_start = b_line;
                let mut deletes = Vec::new();
                let mut inserts = Vec::new();

                while idx < edits.len() {
                    match &edits[idx] {
                        Edit::Delete(line) => {
                            deletes.push(line.clone());
                            a_line += 1;
                            idx += 1;
                        }
                        Edit::Insert(line) => {
                            inserts.push(line.clone());
                            b_line += 1;
                            idx += 1;
                        }
                        Edit::Equal(_) => break,
                    }
                }

                // --- Determine operation and format header ---
                let a_end = a_start + deletes.len().saturating_sub(1);
                let b_end = b_start + inserts.len().saturating_sub(1);

                let a_range = if deletes.len() == 1 {
                    format!("{}", a_start)
                } else if deletes.is_empty() {
                    format!("{}", a_start.saturating_sub(1))
                } else {
                    format!("{},{}", a_start, a_end)
                };

                let b_range = if inserts.len() == 1 {
                    format!("{}", b_start)
                } else if inserts.is_empty() {
                    format!("{}", b_start.saturating_sub(1))
                } else {
                    format!("{},{}", b_start, b_end)
                };

                if !deletes.is_empty() && !inserts.is_empty() {
                    output.push_str(&format!("{}c{}\n", a_range, b_range));
                } else if !deletes.is_empty() {
                    output.push_str(&format!("{}d{}\n", a_range, b_range));
                } else {
                    output.push_str(&format!("{}a{}\n", a_range, b_range));
                }

                for line in &deletes {
                    output.push_str(&format!("< {}\n", line));
                }
                if !deletes.is_empty() && !inserts.is_empty() {
                    output.push_str("---\n");
                }
                for line in &inserts {
                    output.push_str(&format!("> {}\n", line));
                }
            }
        }
    }

    output
}

/// Format edits in unified diff output.
///
/// Unified diff output looks like:
///
/// ```text
///     --- file_a
///     +++ file_b
///     @@ -1,3 +1,3 @@
///      context line
///     -deleted line
///     +inserted line
///      context line
/// ```
///
/// The `@@ -a,len +b,len @@` header tells you what line range changed
/// in each file. Lines prefixed with `-` come from file A, `+` from B,
/// and ` ` (space) are context lines present in both.
pub fn format_unified(edits: &[Edit], file_a: &str, file_b: &str, context: usize) -> String {
    let mut output = String::new();
    output.push_str(&format!("--- {}\n", file_a));
    output.push_str(&format!("+++ {}\n", file_b));

    // --- Identify hunks ---
    // A hunk is a group of changes surrounded by context lines.
    // We find all change positions, then group nearby ones.
    let change_positions: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e, Edit::Equal(_)))
        .map(|(i, _)| i)
        .collect();

    if change_positions.is_empty() {
        return String::new();
    }

    // Group changes into hunks: changes within `context` lines of each
    // other belong to the same hunk.
    let mut hunks: Vec<(usize, usize)> = Vec::new(); // (start_idx, end_idx) in edits
    let mut hunk_start = change_positions[0].saturating_sub(context);
    let mut hunk_end = (change_positions[0] + context).min(edits.len() - 1);

    for &pos in &change_positions[1..] {
        let new_start = pos.saturating_sub(context);
        if new_start <= hunk_end + 1 {
            // Merge with current hunk
            hunk_end = (pos + context).min(edits.len() - 1);
        } else {
            hunks.push((hunk_start, hunk_end));
            hunk_start = new_start;
            hunk_end = (pos + context).min(edits.len() - 1);
        }
    }
    hunks.push((hunk_start, hunk_end));

    // --- Format each hunk ---
    for (start, end) in hunks {
        // Count lines in each file for this hunk
        let mut a_line = 1usize;
        let mut b_line = 1usize;
        // Advance a_line and b_line to the start of this hunk
        for edit in edits.iter().take(start) {
            match edit {
                Edit::Equal(_) | Edit::Delete(_) => a_line += 1,
                _ => {}
            }
            match edit {
                Edit::Equal(_) | Edit::Insert(_) => b_line += 1,
                _ => {}
            }
        }

        let a_start = a_line;
        let b_start = b_line;
        let mut a_count = 0usize;
        let mut b_count = 0usize;

        for edit in edits.iter().take(end + 1).skip(start) {
            match edit {
                Edit::Equal(_) => { a_count += 1; b_count += 1; }
                Edit::Delete(_) => { a_count += 1; }
                Edit::Insert(_) => { b_count += 1; }
            }
        }

        output.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            a_start, a_count, b_start, b_count
        ));

        for edit in edits.iter().take(end + 1).skip(start) {
            match edit {
                Edit::Equal(line) => output.push_str(&format!(" {}\n", line)),
                Edit::Delete(line) => output.push_str(&format!("-{}\n", line)),
                Edit::Insert(line) => output.push_str(&format!("+{}\n", line)),
            }
        }
    }

    output
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compare two files and return the diff output as a string.
///
/// # Workflow
///
/// ```text
///     1. Read both files
///     2. Split into lines
///     3. (Optional) Filter blank lines if -B
///     4. Compute LCS-based diff
///     5. Format according to output mode
/// ```
///
/// # Returns
///
/// - `Ok(Some(output))` if files differ
/// - `Ok(None)` if files are identical
/// - `Err(message)` if a file cannot be read
pub fn diff_files(file_a: &str, file_b: &str, opts: &DiffOptions) -> Result<Option<String>, String> {
    let content_a = fs::read_to_string(file_a)
        .map_err(|e| format!("diff: {}: {}", file_a, e))?;
    let content_b = fs::read_to_string(file_b)
        .map_err(|e| format!("diff: {}: {}", file_b, e))?;

    diff_contents(&content_a, &content_b, file_a, file_b, opts)
}

/// Compare two strings and return the diff output.
///
/// This is the testable core — it takes content strings instead of
/// file paths, making it easy to test without touching the filesystem.
pub fn diff_contents(
    content_a: &str,
    content_b: &str,
    name_a: &str,
    name_b: &str,
    opts: &DiffOptions,
) -> Result<Option<String>, String> {
    let mut lines_a: Vec<&str> = content_a.lines().collect();
    let mut lines_b: Vec<&str> = content_b.lines().collect();

    // --- Filter blank lines if -B ---
    if opts.ignore_blank_lines {
        lines_a.retain(|l| !l.trim().is_empty());
        lines_b.retain(|l| !l.trim().is_empty());
    }

    // --- Compute the edit script ---
    let edits = compute_diff(&lines_a, &lines_b, opts);

    // --- Check if files are identical ---
    let has_changes = edits.iter().any(|e| !matches!(e, Edit::Equal(_)));
    if !has_changes {
        return Ok(None);
    }

    // --- Brief mode: just report that files differ ---
    if opts.brief {
        return Ok(Some(format!(
            "Files {} and {} differ\n",
            name_a, name_b
        )));
    }

    // --- Format the output ---
    let output = if let Some(ctx) = opts.unified_context {
        format_unified(&edits, name_a, name_b, ctx)
    } else if let Some(_ctx) = opts.context_lines {
        // Context format is similar to unified but with different markers.
        // For simplicity, we reuse unified format as a base.
        format_unified(&edits, name_a, name_b, _ctx)
    } else {
        format_normal(&edits)
    };

    Ok(Some(output))
}

/// Recursively compare two directories.
///
/// This walks both directory trees in sorted order and compares
/// matching files. Files that exist in only one directory are
/// reported as "Only in ...".
pub fn diff_directories(
    dir_a: &str,
    dir_b: &str,
    opts: &DiffOptions,
) -> Result<String, String> {
    let path_a = Path::new(dir_a);
    let path_b = Path::new(dir_b);

    if !path_a.is_dir() {
        return Err(format!("diff: {}: Not a directory", dir_a));
    }
    if !path_b.is_dir() {
        return Err(format!("diff: {}: Not a directory", dir_b));
    }

    let mut output = String::new();

    // Collect entries from both directories
    let entries_a = list_dir_sorted(path_a)?;
    let entries_b = list_dir_sorted(path_b)?;

    // Merge-walk both sorted lists
    let mut i = 0;
    let mut j = 0;

    while i < entries_a.len() || j < entries_b.len() {
        let cmp = match (entries_a.get(i), entries_b.get(j)) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => break,
        };

        match cmp {
            std::cmp::Ordering::Less => {
                output.push_str(&format!("Only in {}: {}\n", dir_a, entries_a[i]));
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                output.push_str(&format!("Only in {}: {}\n", dir_b, entries_b[j]));
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                let child_a = path_a.join(&entries_a[i]);
                let child_b = path_b.join(&entries_b[j]);

                if child_a.is_dir() && child_b.is_dir() && opts.recursive {
                    let sub_output = diff_directories(
                        &child_a.to_string_lossy(),
                        &child_b.to_string_lossy(),
                        opts,
                    )?;
                    output.push_str(&sub_output);
                } else if child_a.is_file() && child_b.is_file() {
                    if let Ok(Some(diff_output)) = diff_files(
                        &child_a.to_string_lossy(),
                        &child_b.to_string_lossy(),
                        opts,
                    ) {
                        output.push_str(&format!(
                            "diff {} {}\n",
                            child_a.display(),
                            child_b.display()
                        ));
                        output.push_str(&diff_output);
                    }
                }

                i += 1;
                j += 1;
            }
        }
    }

    Ok(output)
}

/// List directory entries sorted alphabetically.
fn list_dir_sorted(dir: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("diff: {}: {}", dir.display(), e))?;

    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    names.sort();
    Ok(names)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_files_produce_no_diff() {
        let result = diff_contents("a\nb\nc\n", "a\nb\nc\n", "a.txt", "b.txt", &DiffOptions::default());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn single_line_deletion() {
        let edits = compute_diff(&["a", "b", "c"], &["a", "c"], &DiffOptions::default());
        assert!(edits.contains(&Edit::Delete("b".to_string())));
    }

    #[test]
    fn single_line_insertion() {
        let edits = compute_diff(&["a", "c"], &["a", "b", "c"], &DiffOptions::default());
        assert!(edits.contains(&Edit::Insert("b".to_string())));
    }

    #[test]
    fn ignore_case_treats_lines_as_equal() {
        let opts = DiffOptions { ignore_case: true, ..Default::default() };
        let result = diff_contents("Hello\n", "hello\n", "a", "b", &opts);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn ignore_all_space() {
        let opts = DiffOptions { ignore_all_space: true, ..Default::default() };
        let result = diff_contents("a b c\n", "abc\n", "a", "b", &opts);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn ignore_space_change() {
        let opts = DiffOptions { ignore_space_change: true, ..Default::default() };
        let result = diff_contents("a  b  c\n", "a b c\n", "a", "b", &opts);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn ignore_blank_lines() {
        let opts = DiffOptions { ignore_blank_lines: true, ..Default::default() };
        let result = diff_contents("a\n\nb\n", "a\nb\n", "a", "b", &opts);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn brief_mode() {
        let opts = DiffOptions { brief: true, ..Default::default() };
        let result = diff_contents("a\n", "b\n", "file1", "file2", &opts);
        assert_eq!(result.unwrap(), Some("Files file1 and file2 differ\n".to_string()));
    }

    #[test]
    fn normal_format_delete() {
        let edits = vec![
            Edit::Equal("a".into()),
            Edit::Delete("b".into()),
            Edit::Equal("c".into()),
        ];
        let output = format_normal(&edits);
        assert!(output.contains("< b"));
    }

    #[test]
    fn normal_format_insert() {
        let edits = vec![
            Edit::Equal("a".into()),
            Edit::Insert("b".into()),
            Edit::Equal("c".into()),
        ];
        let output = format_normal(&edits);
        assert!(output.contains("> b"));
    }

    #[test]
    fn unified_format_has_headers() {
        let edits = vec![
            Edit::Equal("a".into()),
            Edit::Delete("b".into()),
            Edit::Insert("c".into()),
        ];
        let output = format_unified(&edits, "old.txt", "new.txt", 3);
        assert!(output.contains("--- old.txt"));
        assert!(output.contains("+++ new.txt"));
        assert!(output.contains("@@"));
    }

    #[test]
    fn normalize_preserves_content_by_default() {
        let opts = DiffOptions::default();
        assert_eq!(normalize_line("Hello World", &opts), "Hello World");
    }

    #[test]
    fn completely_different_files() {
        let result = diff_contents("a\nb\nc\n", "x\ny\nz\n", "a", "b", &DiffOptions::default());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn empty_files_are_identical() {
        let result = diff_contents("", "", "a", "b", &DiffOptions::default());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn one_empty_one_not() {
        let result = diff_contents("", "hello\n", "a", "b", &DiffOptions::default());
        assert!(result.unwrap().is_some());
    }
}
