//! # sort — Sort Lines of Text Files
//!
//! This module implements the business logic for the `sort` command.
//! `sort` reads lines from input and writes them to standard output,
//! sorted according to the specified ordering.
//!
//! ## Sorting Modes
//!
//! ```text
//!     Mode            Flag    Description
//!     ──────────────  ─────   ─────────────────────────────────────
//!     Lexicographic   (none)  Default: compare bytes left to right
//!     Numeric         -n      Parse as integers, compare numerically
//!     General numeric -g      Parse as floats, compare numerically
//!     Human numeric   -h      Parse suffixes like K, M, G (1024-based)
//!     Month           -M      JAN < FEB < ... < DEC
//!     Version         -V      Natural sort of version numbers
//! ```
//!
//! ## Modifiers
//!
//! ```text
//!     -r    Reverse the comparison
//!     -f    Fold lowercase to uppercase (case-insensitive)
//!     -d    Dictionary order: only blanks and alphanumeric chars
//!     -i    Ignore non-printing characters
//!     -b    Ignore leading blanks
//!     -u    Unique: output only the first of equal lines
//!     -s    Stable sort (preserve input order for equal elements)
//! ```
//!
//! ## How Sorting Works
//!
//! At its core, sorting is about defining a total order on lines.
//! We build a comparison function from the flags, then feed it
//! to Rust's built-in sort algorithm. The key insight is that
//! all the flags just modify the comparison function — the
//! actual sorting algorithm stays the same.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Options that control how `sort_lines` compares and filters lines.
///
/// Each field corresponds to a command-line flag. The defaults match
/// the behavior of `sort` with no flags: lexicographic, ascending,
/// case-sensitive, including duplicates.
#[derive(Debug, Clone, Default)]
pub struct SortOptions {
    /// Reverse the sort order (-r).
    pub reverse: bool,
    /// Compare according to string numerical value (-n).
    pub numeric: bool,
    /// Compare according to general numerical value (-g).
    /// Unlike -n, this handles floating point and scientific notation.
    pub general_numeric: bool,
    /// Compare human-readable numbers like 2K, 1G (-h).
    pub human_numeric: bool,
    /// Compare abbreviated month names (-M).
    pub month: bool,
    /// Natural sort of version numbers (-V).
    pub version: bool,
    /// Fold lowercase to uppercase for comparison (-f).
    pub ignore_case: bool,
    /// Consider only blanks and alphanumeric characters (-d).
    pub dictionary_order: bool,
    /// Consider only printable characters (-i).
    pub ignore_nonprinting: bool,
    /// Ignore leading blanks (-b).
    pub ignore_leading_blanks: bool,
    /// Output only the first of equal lines (-u).
    pub unique: bool,
    /// Stable sort — preserve input order for equal elements (-s).
    pub stable: bool,
}

/// Sort a slice of lines according to the given options.
///
/// This is the core business logic. It takes a slice of lines (without
/// trailing newlines) and returns a new Vec of sorted lines.
///
/// # How the Comparison Pipeline Works
///
/// ```text
///     Input line ──→ [transform] ──→ [compare] ──→ [maybe reverse]
///
///     1. Transform: apply -b, -d, -i, -f to get a "sort key"
///     2. Compare: use the appropriate comparison (-n, -g, -M, etc.)
///     3. Reverse: if -r is set, flip the result
/// ```
///
/// # Example
///
/// ```text
///     let lines = vec!["banana".into(), "apple".into(), "cherry".into()];
///     let opts = SortOptions::default();
///     let sorted = sort_lines(&lines, &opts);
///     // => ["apple", "banana", "cherry"]
/// ```
pub fn sort_lines(lines: &[String], opts: &SortOptions) -> Vec<String> {
    let mut result: Vec<String> = lines.to_vec();

    // --- Choose between stable and unstable sort ---
    // Stable sort preserves the relative order of equal elements.
    // Unstable sort is faster but doesn't guarantee preservation.
    // When -s is set, we always use stable sort.
    // We actually always use stable sort in this implementation for
    // correctness, but the flag is here for POSIX compatibility.
    result.sort_by(|a, b| {
        let ordering = compare_lines(a, b, opts);
        if opts.reverse {
            ordering.reverse()
        } else {
            ordering
        }
    });

    // --- Deduplicate if -u is set ---
    // With -u, we keep only the first line of each group of equal lines.
    // "Equal" is defined by the same comparison function used for sorting.
    if opts.unique {
        result.dedup_by(|a, b| compare_lines(a, b, opts) == std::cmp::Ordering::Equal);
    }

    result
}

// ---------------------------------------------------------------------------
// Comparison Logic
// ---------------------------------------------------------------------------

/// Compare two lines according to the sort options.
///
/// This is the heart of the sort tool. Each sorting mode defines
/// a different way to interpret the line content for comparison.
fn compare_lines(a: &str, b: &str, opts: &SortOptions) -> std::cmp::Ordering {
    // --- Step 1: Transform lines into sort keys ---
    // Apply text transformations before comparison.
    let key_a = make_sort_key(a, opts);
    let key_b = make_sort_key(b, opts);

    // --- Step 2: Compare using the appropriate mode ---
    if opts.numeric {
        compare_numeric(&key_a, &key_b)
    } else if opts.general_numeric {
        compare_general_numeric(&key_a, &key_b)
    } else if opts.human_numeric {
        compare_human_numeric(&key_a, &key_b)
    } else if opts.month {
        compare_month(&key_a, &key_b)
    } else if opts.version {
        compare_version(&key_a, &key_b)
    } else {
        // Default: lexicographic comparison
        key_a.cmp(&key_b)
    }
}

/// Build a sort key from a line by applying text transformations.
///
/// The transformations are applied in this order:
/// 1. Strip leading blanks (-b)
/// 2. Keep only dictionary characters (-d)
/// 3. Keep only printable characters (-i)
/// 4. Fold to uppercase (-f)
fn make_sort_key(line: &str, opts: &SortOptions) -> String {
    let mut key = line.to_string();

    // -b: ignore leading blanks
    if opts.ignore_leading_blanks {
        key = key.trim_start().to_string();
    }

    // -d: dictionary order — keep only blanks and alphanumeric
    if opts.dictionary_order {
        key = key
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
    }

    // -i: ignore non-printing characters (keep only ASCII 32-126)
    if opts.ignore_nonprinting {
        key = key.chars().filter(|c| (' '..='~').contains(c)).collect();
    }

    // -f: fold lowercase to uppercase
    if opts.ignore_case {
        key = key.to_uppercase();
    }

    key
}

/// Compare two strings as integers.
///
/// Leading whitespace is ignored. Non-numeric strings are treated as 0.
///
/// ```text
///     "  42"  vs "7"    → 42 > 7  → Greater
///     "abc"   vs "123"  → 0 < 123 → Less
///     ""      vs "0"    → 0 == 0  → Equal
/// ```
fn compare_numeric(a: &str, b: &str) -> std::cmp::Ordering {
    let na = parse_leading_number(a);
    let nb = parse_leading_number(b);
    na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
}

/// Parse the leading numeric portion of a string as f64.
///
/// This handles integers, negative numbers, and leading whitespace.
/// If the string doesn't start with a number, returns 0.0.
fn parse_leading_number(s: &str) -> f64 {
    let trimmed = s.trim_start();
    // Find the longest prefix that parses as a number
    let mut end = 0;
    let chars: Vec<char> = trimmed.chars().collect();
    if end < chars.len() && (chars[end] == '-' || chars[end] == '+') {
        end += 1;
    }
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0
        || (end == 1 && (chars.first() == Some(&'-') || chars.first() == Some(&'+')))
    {
        return 0.0;
    }
    let num_str: String = chars[..end].iter().collect();
    num_str.parse::<f64>().unwrap_or(0.0)
}

/// Compare two strings as floating-point numbers.
///
/// This handles scientific notation (1.5e10), infinity, and NaN.
/// NaN values sort before everything else (matching GNU sort behavior).
fn compare_general_numeric(a: &str, b: &str) -> std::cmp::Ordering {
    let na = a.trim().parse::<f64>().unwrap_or(f64::NAN);
    let nb = b.trim().parse::<f64>().unwrap_or(f64::NAN);

    // NaN sorts before everything else
    match (na.is_nan(), nb.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
    }
}

/// Compare human-readable sizes (e.g., "2K", "1.5G", "100M").
///
/// Suffixes and their multipliers (base 1024):
/// ```text
///     K = 1024^1, M = 1024^2, G = 1024^3, T = 1024^4
///     P = 1024^5, E = 1024^6
/// ```
fn compare_human_numeric(a: &str, b: &str) -> std::cmp::Ordering {
    let na = parse_human_size(a);
    let nb = parse_human_size(b);
    na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
}

/// Parse a human-readable size string into bytes.
///
/// ```text
///     "1K"   → 1024.0
///     "2.5M" → 2621440.0
///     "100"  → 100.0
///     "abc"  → 0.0
/// ```
fn parse_human_size(s: &str) -> f64 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let last = trimmed.chars().last().unwrap();
    let multiplier = match last.to_ascii_uppercase() {
        'K' => 1024.0_f64,
        'M' => 1024.0_f64.powi(2),
        'G' => 1024.0_f64.powi(3),
        'T' => 1024.0_f64.powi(4),
        'P' => 1024.0_f64.powi(5),
        'E' => 1024.0_f64.powi(6),
        _ => {
            return trimmed.parse::<f64>().unwrap_or(0.0);
        }
    };

    let num_part = &trimmed[..trimmed.len() - 1];
    let num = num_part.parse::<f64>().unwrap_or(0.0);
    num * multiplier
}

/// Compare abbreviated month names.
///
/// The ordering is: (unknown) < JAN < FEB < ... < DEC
/// Case-insensitive. Non-month strings all compare equal (as "unknown").
fn compare_month(a: &str, b: &str) -> std::cmp::Ordering {
    let ma = month_rank(a.trim());
    let mb = month_rank(b.trim());
    ma.cmp(&mb)
}

/// Map an abbreviated month name to its rank (0 = unknown, 1 = JAN, ..., 12 = DEC).
fn month_rank(s: &str) -> u8 {
    let upper: String = s.to_uppercase();
    let prefix = if upper.len() >= 3 {
        &upper[..3]
    } else {
        &upper
    };
    match prefix {
        "JAN" => 1,
        "FEB" => 2,
        "MAR" => 3,
        "APR" => 4,
        "MAY" => 5,
        "JUN" => 6,
        "JUL" => 7,
        "AUG" => 8,
        "SEP" => 9,
        "OCT" => 10,
        "NOV" => 11,
        "DEC" => 12,
        _ => 0,
    }
}

/// Compare version strings naturally.
///
/// Version sort splits strings into alternating runs of digits and
/// non-digits, then compares digit runs numerically and non-digit
/// runs lexicographically.
///
/// ```text
///     "file2"  vs "file10"  → file2 < file10 (2 < 10 numerically)
///     "1.9.0"  vs "1.10.0"  → 1.9.0 < 1.10.0
/// ```
fn compare_version(a: &str, b: &str) -> std::cmp::Ordering {
    let parts_a = split_version(a);
    let parts_b = split_version(b);

    for (pa, pb) in parts_a.iter().zip(parts_b.iter()) {
        let ord = match (pa.parse::<u64>(), pb.parse::<u64>()) {
            (Ok(na), Ok(nb)) => na.cmp(&nb),
            _ => pa.cmp(pb),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }

    parts_a.len().cmp(&parts_b.len())
}

/// Split a string into alternating runs of digits and non-digits.
///
/// ```text
///     "file10.txt" → ["file", "10", ".", "txt"]
/// ```
fn split_version(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_digits = false;

    for ch in s.chars() {
        let is_digit = ch.is_ascii_digit();
        if !current.is_empty() && is_digit != in_digits {
            parts.push(current.clone());
            current.clear();
        }
        in_digits = is_digit;
        current.push(ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lexicographic_sort() {
        let lines = vec!["banana".into(), "apple".into(), "cherry".into()];
        let result = sort_lines(&lines, &SortOptions::default());
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn reverse_sort() {
        let lines = vec!["a".into(), "c".into(), "b".into()];
        let opts = SortOptions { reverse: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["c", "b", "a"]);
    }

    #[test]
    fn numeric_sort() {
        let lines = vec!["10".into(), "2".into(), "1".into(), "20".into()];
        let opts = SortOptions { numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["1", "2", "10", "20"]);
    }

    #[test]
    fn numeric_sort_with_non_numbers() {
        let lines = vec!["abc".into(), "5".into(), "2".into()];
        let opts = SortOptions { numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["abc", "2", "5"]);
    }

    #[test]
    fn human_numeric_sort() {
        let lines = vec!["1G".into(), "2K".into(), "3M".into()];
        let opts = SortOptions { human_numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["2K", "3M", "1G"]);
    }

    #[test]
    fn month_sort() {
        let lines = vec!["MAR".into(), "JAN".into(), "FEB".into()];
        let opts = SortOptions { month: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["JAN", "FEB", "MAR"]);
    }

    #[test]
    fn version_sort() {
        let lines = vec!["file10".into(), "file2".into(), "file1".into()];
        let opts = SortOptions { version: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["file1", "file2", "file10"]);
    }

    #[test]
    fn unique() {
        let lines = vec!["a".into(), "b".into(), "a".into(), "c".into()];
        let opts = SortOptions { unique: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn ignore_case_sort() {
        let lines = vec!["Banana".into(), "apple".into(), "Cherry".into()];
        let opts = SortOptions { ignore_case: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["apple", "Banana", "Cherry"]);
    }

    #[test]
    fn ignore_leading_blanks() {
        let lines = vec!["  b".into(), "a".into(), "   c".into()];
        let opts = SortOptions { ignore_leading_blanks: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["a", "  b", "   c"]);
    }

    #[test]
    fn empty_input() {
        let lines: Vec<String> = vec![];
        let result = sort_lines(&lines, &SortOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn general_numeric_sort() {
        let lines = vec!["1.5e2".into(), "100".into(), "50".into()];
        let opts = SortOptions { general_numeric: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        assert_eq!(result, vec!["50", "100", "1.5e2"]);
    }

    #[test]
    fn parse_human_size_values() {
        assert_eq!(parse_human_size("1K"), 1024.0);
        assert_eq!(parse_human_size("2M"), 2.0 * 1024.0 * 1024.0);
        assert_eq!(parse_human_size("100"), 100.0);
        assert_eq!(parse_human_size(""), 0.0);
    }

    #[test]
    fn month_rank_values() {
        assert_eq!(month_rank("JAN"), 1);
        assert_eq!(month_rank("dec"), 12);
        assert_eq!(month_rank("xyz"), 0);
    }

    #[test]
    fn split_version_parts() {
        assert_eq!(split_version("file10.txt"), vec!["file", "10", ".txt"]);
        assert_eq!(split_version("abc"), vec!["abc"]);
        assert_eq!(split_version("123"), vec!["123"]);
    }

    #[test]
    fn dictionary_order() {
        let lines = vec!["b-c".into(), "a.b".into(), "a c".into()];
        let opts = SortOptions { dictionary_order: true, ..Default::default() };
        let result = sort_lines(&lines, &opts);
        // With dictionary order, punctuation is stripped
        // "b-c" → "bc", "a.b" → "ab", "a c" → "a c"
        assert_eq!(result, vec!["a c", "a.b", "b-c"]);
    }
}
