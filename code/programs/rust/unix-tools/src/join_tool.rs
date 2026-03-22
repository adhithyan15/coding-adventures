//! # join — Join Lines of Two Files on a Common Field
//!
//! This module implements the business logic for the `join` command.
//! `join` performs a relational join on two sorted files, pairing
//! lines that share a common field value.
//!
//! ## How It Works
//!
//! Think of `join` as a database JOIN operation, but on text files:
//!
//! ```text
//!     File 1:                 File 2:
//!     1 Alice                 1 Engineering
//!     2 Bob                   2 Marketing
//!     3 Charlie               4 Sales
//!
//!     join file1 file2:
//!     1 Alice Engineering     ← key "1" matched
//!     2 Bob Marketing         ← key "2" matched
//!                             ← "3" and "4" have no match (omitted)
//! ```
//!
//! ## Requirements
//!
//! Both files must be sorted on the join field. This is because `join`
//! uses a merge-join algorithm (like merge sort's merge step) that
//! walks through both files in lockstep. Unsorted input produces
//! incorrect results.
//!
//! ## Merge-Join Algorithm
//!
//! ```text
//!     i = 0, j = 0
//!     while i < lines1.len() and j < lines2.len():
//!         key1 = field(lines1[i], field1)
//!         key2 = field(lines2[j], field2)
//!         if key1 == key2:
//!             emit joined line
//!             advance both
//!         elif key1 < key2:
//!             emit unpaired line from file1 (if -a 1)
//!             advance i
//!         else:
//!             emit unpaired line from file2 (if -a 2)
//!             advance j
//! ```

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `join_lines` behaves.
///
/// ```text
///     Flag           Field            Effect
///     ─────────────  ──────────────   ──────────────────────────────
///     -1 FIELD       field1           Join on this field of file 1 (1-based)
///     -2 FIELD       field2           Join on this field of file 2 (1-based)
///     -t CHAR        separator        Use CHAR as field separator
///     -a FILENUM     unpaired         Also print unpaired lines
///     -v FILENUM     only_unpaired    Print ONLY unpaired lines
///     -e EMPTY       empty            Replace missing fields with EMPTY
///     -i             ignore_case      Case-insensitive comparison
/// ```
#[derive(Debug, Clone)]
pub struct JoinOptions {
    /// Which field to join on in file 1 (1-based index, default 1).
    pub field1: usize,
    /// Which field to join on in file 2 (1-based index, default 1).
    pub field2: usize,
    /// Field separator. `None` means whitespace (the default).
    pub separator: Option<char>,
    /// Also print unpaired lines from these files.
    /// Contains 1, 2, or both.
    pub unpaired: Vec<usize>,
    /// Print ONLY unpaired lines from this file (suppresses joined output).
    /// `None` means print joined output normally.
    pub only_unpaired: Option<usize>,
    /// String to use for missing fields.
    pub empty: String,
    /// Case-insensitive key comparison.
    pub ignore_case: bool,
}

impl Default for JoinOptions {
    fn default() -> Self {
        JoinOptions {
            field1: 1,
            field2: 1,
            separator: None,
            unpaired: Vec::new(),
            only_unpaired: None,
            empty: String::new(),
            ignore_case: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Join two sets of sorted lines on a common field.
///
/// This implements the merge-join algorithm. Both `lines1` and `lines2`
/// must be sorted on their respective join fields.
///
/// # Returns
///
/// A vector of joined output lines. Each output line contains:
/// - The common join field
/// - Remaining fields from file 1
/// - Remaining fields from file 2
///
/// # Example
///
/// ```text
///     lines1 = ["1 Alice", "2 Bob"]
///     lines2 = ["1 Engineering", "2 Marketing"]
///     opts = default (join on field 1)
///     result = ["1 Alice Engineering", "2 Bob Marketing"]
/// ```
pub fn join_lines(
    lines1: &[String],
    lines2: &[String],
    opts: &JoinOptions,
) -> Vec<String> {
    let sep = opts.separator.unwrap_or(' ');
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < lines1.len() && j < lines2.len() {
        let key1 = get_field(&lines1[i], opts.field1, opts.separator);
        let key2 = get_field(&lines2[j], opts.field2, opts.separator);

        let cmp = compare_keys(&key1, &key2, opts.ignore_case);

        match cmp {
            std::cmp::Ordering::Equal => {
                // --- Keys match: handle potential many-to-many join ---
                // Collect all lines from file2 with the same key
                let mut j2 = j;
                while j2 < lines2.len() {
                    let k2 = get_field(&lines2[j2], opts.field2, opts.separator);
                    if compare_keys(&k2, &key1, opts.ignore_case) != std::cmp::Ordering::Equal {
                        break;
                    }
                    j2 += 1;
                }

                // For each line in file1 with this key, pair with each file2 line
                let mut i2 = i;
                while i2 < lines1.len() {
                    let k1 = get_field(&lines1[i2], opts.field1, opts.separator);
                    if compare_keys(&k1, &key1, opts.ignore_case) != std::cmp::Ordering::Equal {
                        break;
                    }

                    for jj in j..j2 {
                        if opts.only_unpaired.is_none() {
                            let joined = format_joined(
                                &key1,
                                &lines1[i2],
                                opts.field1,
                                &lines2[jj],
                                opts.field2,
                                opts.separator,
                                sep,
                            );
                            result.push(joined);
                        }
                    }
                    i2 += 1;
                }

                i = i2;
                j = j2;
            }
            std::cmp::Ordering::Less => {
                // key1 < key2: line from file1 has no match
                if opts.unpaired.contains(&1) || opts.only_unpaired == Some(1) {
                    result.push(lines1[i].clone());
                }
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                // key1 > key2: line from file2 has no match
                if opts.unpaired.contains(&2) || opts.only_unpaired == Some(2) {
                    result.push(lines2[j].clone());
                }
                j += 1;
            }
        }
    }

    // --- Handle remaining lines ---
    while i < lines1.len() {
        if opts.unpaired.contains(&1) || opts.only_unpaired == Some(1) {
            result.push(lines1[i].clone());
        }
        i += 1;
    }

    while j < lines2.len() {
        if opts.unpaired.contains(&2) || opts.only_unpaired == Some(2) {
            result.push(lines2[j].clone());
        }
        j += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/// Extract a 1-based field from a line.
///
/// Fields are separated by the given separator (or whitespace by default).
///
/// ```text
///     get_field("Alice 30 NYC", 2, None) → "30"
///     get_field("a:b:c", 1, Some(':'))   → "a"
///     get_field("short", 5, None)        → ""  (field doesn't exist)
/// ```
fn get_field(line: &str, field_num: usize, separator: Option<char>) -> String {
    let fields: Vec<&str> = match separator {
        Some(sep) => line.split(sep).collect(),
        None => line.split_whitespace().collect(),
    };

    // field_num is 1-based
    if field_num == 0 || field_num > fields.len() {
        return String::new();
    }

    fields[field_num - 1].to_string()
}

/// Compare two join keys, optionally case-insensitive.
fn compare_keys(a: &str, b: &str, ignore_case: bool) -> std::cmp::Ordering {
    if ignore_case {
        a.to_lowercase().cmp(&b.to_lowercase())
    } else {
        a.cmp(b)
    }
}

/// Format a joined output line.
///
/// The output contains:
/// 1. The join key
/// 2. All non-key fields from file 1
/// 3. All non-key fields from file 2
///
/// ```text
///     key = "1"
///     line1 = "1 Alice 30"   (field1 = 1, so "Alice 30" are non-key)
///     line2 = "1 Engineering" (field2 = 1, so "Engineering" is non-key)
///     output = "1 Alice 30 Engineering"
/// ```
fn format_joined(
    key: &str,
    line1: &str,
    field1: usize,
    line2: &str,
    field2: usize,
    input_sep: Option<char>,
    output_sep: char,
) -> String {
    let fields1 = split_fields(line1, input_sep);
    let fields2 = split_fields(line2, input_sep);

    let mut parts = vec![key.to_string()];

    // Add non-key fields from file 1
    for (idx, f) in fields1.iter().enumerate() {
        if idx + 1 != field1 {
            parts.push(f.to_string());
        }
    }

    // Add non-key fields from file 2
    for (idx, f) in fields2.iter().enumerate() {
        if idx + 1 != field2 {
            parts.push(f.to_string());
        }
    }

    parts.join(&output_sep.to_string())
}

/// Split a line into fields.
fn split_fields(line: &str, separator: Option<char>) -> Vec<String> {
    match separator {
        Some(sep) => line.split(sep).map(|s| s.to_string()).collect(),
        None => line.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_join() {
        let lines1: Vec<String> = vec!["1 Alice".into(), "2 Bob".into()];
        let lines2: Vec<String> = vec!["1 Engineering".into(), "2 Marketing".into()];
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(result, vec!["1 Alice Engineering", "2 Bob Marketing"]);
    }

    #[test]
    fn join_with_no_matches() {
        let lines1: Vec<String> = vec!["1 Alice".into(), "3 Charlie".into()];
        let lines2: Vec<String> = vec!["2 Engineering".into(), "4 Sales".into()];
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn join_partial_match() {
        let lines1: Vec<String> = vec!["1 Alice".into(), "2 Bob".into(), "3 Charlie".into()];
        let lines2: Vec<String> = vec!["1 Engineering".into(), "3 Sales".into()];
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(result, vec!["1 Alice Engineering", "3 Charlie Sales"]);
    }

    #[test]
    fn join_with_unpaired_file1() {
        let lines1: Vec<String> = vec!["1 Alice".into(), "2 Bob".into(), "3 Charlie".into()];
        let lines2: Vec<String> = vec!["1 Engineering".into(), "3 Sales".into()];
        let opts = JoinOptions {
            unpaired: vec![1],
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["1 Alice Engineering", "2 Bob", "3 Charlie Sales"]
        );
    }

    #[test]
    fn join_with_unpaired_file2() {
        let lines1: Vec<String> = vec!["1 Alice".into()];
        let lines2: Vec<String> = vec!["1 Engineering".into(), "2 Marketing".into()];
        let opts = JoinOptions {
            unpaired: vec![2],
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["1 Alice Engineering", "2 Marketing"]
        );
    }

    #[test]
    fn join_only_unpaired() {
        let lines1: Vec<String> = vec!["1 Alice".into(), "2 Bob".into()];
        let lines2: Vec<String> = vec!["1 Engineering".into()];
        let opts = JoinOptions {
            only_unpaired: Some(1),
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["2 Bob"]);
    }

    #[test]
    fn join_on_field2() {
        let lines1: Vec<String> = vec!["Alice 1".into(), "Bob 2".into()];
        let lines2: Vec<String> = vec!["Engineering 1".into(), "Marketing 2".into()];
        let opts = JoinOptions {
            field1: 2,
            field2: 2,
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["1 Alice Engineering", "2 Bob Marketing"]
        );
    }

    #[test]
    fn join_with_custom_separator() {
        let lines1: Vec<String> = vec!["1:Alice".into(), "2:Bob".into()];
        let lines2: Vec<String> = vec!["1:Engineering".into(), "2:Marketing".into()];
        let opts = JoinOptions {
            separator: Some(':'),
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(result, vec!["1:Alice:Engineering", "2:Bob:Marketing"]);
    }

    #[test]
    fn join_case_insensitive() {
        let lines1: Vec<String> = vec!["a Alice".into(), "B Bob".into()];
        let lines2: Vec<String> = vec!["A Engineering".into(), "b Marketing".into()];
        let opts = JoinOptions {
            ignore_case: true,
            ..Default::default()
        };
        let result = join_lines(&lines1, &lines2, &opts);
        assert_eq!(
            result,
            vec!["a Alice Engineering", "B Bob Marketing"]
        );
    }

    #[test]
    fn join_empty_inputs() {
        let empty: Vec<String> = vec![];
        let lines: Vec<String> = vec!["1 Alice".into()];
        assert!(join_lines(&empty, &lines, &JoinOptions::default()).is_empty());
        assert!(join_lines(&lines, &empty, &JoinOptions::default()).is_empty());
        assert!(join_lines(&empty, &empty, &JoinOptions::default()).is_empty());
    }

    #[test]
    fn get_field_basic() {
        assert_eq!(get_field("Alice 30 NYC", 1, None), "Alice");
        assert_eq!(get_field("Alice 30 NYC", 2, None), "30");
        assert_eq!(get_field("Alice 30 NYC", 3, None), "NYC");
    }

    #[test]
    fn get_field_with_separator() {
        assert_eq!(get_field("a:b:c", 1, Some(':')), "a");
        assert_eq!(get_field("a:b:c", 2, Some(':')), "b");
        assert_eq!(get_field("a:b:c", 3, Some(':')), "c");
    }

    #[test]
    fn get_field_out_of_range() {
        assert_eq!(get_field("hello", 5, None), "");
        assert_eq!(get_field("hello", 0, None), "");
    }

    #[test]
    fn join_many_to_many() {
        let lines1: Vec<String> = vec!["1 A".into(), "1 B".into()];
        let lines2: Vec<String> = vec!["1 X".into(), "1 Y".into()];
        let result = join_lines(&lines1, &lines2, &JoinOptions::default());
        assert_eq!(
            result,
            vec!["1 A X", "1 A Y", "1 B X", "1 B Y"]
        );
    }
}
