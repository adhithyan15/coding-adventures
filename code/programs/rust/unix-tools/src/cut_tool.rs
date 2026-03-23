//! # cut — Remove Sections from Each Line
//!
//! This module implements the business logic for the `cut` command.
//! `cut` extracts selected portions of each line from input.
//!
//! ## Three Selection Modes
//!
//! `cut` operates in one of three mutually exclusive modes:
//!
//! ```text
//!     Mode         Flag    Description
//!     ───────────  ─────   ──────────────────────────────
//!     Bytes        -b      Select specific byte positions
//!     Characters   -c      Select specific character positions
//!     Fields       -f      Select specific field numbers
//! ```
//!
//! ## Range Lists
//!
//! All three modes accept a "range list" — a comma-separated list
//! of ranges that specify which positions to extract:
//!
//! ```text
//!     Syntax      Meaning
//!     ──────      ───────────────────────────────
//!     N           Just position N
//!     N-M         Positions N through M (inclusive)
//!     N-          Position N through end of line
//!     -M          Position 1 through M
//! ```
//!
//! Positions are 1-based (first byte/char/field is position 1).
//!
//! ## Field Mode Details
//!
//! In field mode (-f), the delimiter defaults to TAB. Fields are
//! separated by exactly one delimiter character (unlike `awk`, which
//! treats runs of whitespace as a single separator).
//!
//! ```text
//!     Input:  "alice\t42\tengineer"
//!     cut -f2:  "42"
//!     cut -f1,3: "alice\tengineer"
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A range specification for selecting bytes, characters, or fields.
///
/// Ranges are 1-based and inclusive on both ends.
#[derive(Debug, Clone, PartialEq)]
pub enum Range {
    /// A single position: `N`
    Single(usize),
    /// A closed range: `N-M`
    Closed(usize, usize),
    /// From position N to end of line: `N-`
    From(usize),
    /// From start to position M: `-M`
    To(usize),
}

/// Options for the cut operation.
#[derive(Debug, Clone)]
pub struct CutOptions {
    /// The parsed range list.
    pub ranges: Vec<Range>,
    /// Field delimiter (default: TAB). Only used in field mode.
    pub delimiter: char,
    /// Output delimiter. Defaults to the input delimiter.
    pub output_delimiter: Option<String>,
    /// If true, suppress lines that don't contain the delimiter (-s).
    pub only_delimited: bool,
    /// If true, complement the selected set.
    pub complement: bool,
}

impl Default for CutOptions {
    fn default() -> Self {
        Self {
            ranges: Vec::new(),
            delimiter: '\t',
            output_delimiter: None,
            only_delimited: false,
            complement: false,
        }
    }
}

/// Parse a range list string like "1,3-5,7-" into a Vec of Range.
///
/// # Grammar
///
/// ```text
///     range_list := range (',' range)*
///     range      := N | N'-'M | N'-' | '-'M
///     N, M       := positive integer
/// ```
///
/// # Examples
///
/// ```text
///     "1"     → [Single(1)]
///     "1,3"   → [Single(1), Single(3)]
///     "1-3"   → [Closed(1, 3)]
///     "3-"    → [From(3)]
///     "-5"    → [To(5)]
/// ```
pub fn parse_range_list(spec: &str) -> Result<Vec<Range>, String> {
    let mut ranges = Vec::new();

    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err("cut: invalid range: empty range".to_string());
        }

        if let Some(dash_pos) = part.find('-') {
            let left = &part[..dash_pos];
            let right = &part[dash_pos + 1..];

            match (left.is_empty(), right.is_empty()) {
                // "-M"
                (true, false) => {
                    let m = right.parse::<usize>()
                        .map_err(|_| format!("cut: invalid range: '{}'", part))?;
                    if m == 0 {
                        return Err("cut: fields and positions are numbered from 1".to_string());
                    }
                    ranges.push(Range::To(m));
                }
                // "N-"
                (false, true) => {
                    let n = left.parse::<usize>()
                        .map_err(|_| format!("cut: invalid range: '{}'", part))?;
                    if n == 0 {
                        return Err("cut: fields and positions are numbered from 1".to_string());
                    }
                    ranges.push(Range::From(n));
                }
                // "N-M"
                (false, false) => {
                    let n = left.parse::<usize>()
                        .map_err(|_| format!("cut: invalid range: '{}'", part))?;
                    let m = right.parse::<usize>()
                        .map_err(|_| format!("cut: invalid range: '{}'", part))?;
                    if n == 0 || m == 0 {
                        return Err("cut: fields and positions are numbered from 1".to_string());
                    }
                    ranges.push(Range::Closed(n, m));
                }
                // "-" alone
                (true, true) => {
                    return Err("cut: invalid range: '-'".to_string());
                }
            }
        } else {
            let n = part.parse::<usize>()
                .map_err(|_| format!("cut: invalid range: '{}'", part))?;
            if n == 0 {
                return Err("cut: fields and positions are numbered from 1".to_string());
            }
            ranges.push(Range::Single(n));
        }
    }

    Ok(ranges)
}

/// Cut characters from a line based on character ranges.
///
/// Characters are 1-indexed. This operates on Unicode code points,
/// not bytes — so multi-byte characters are handled correctly.
///
/// # Example
///
/// ```text
///     cut_characters("hello world", ranges=[Closed(1,5)]) → "hello"
/// ```
pub fn cut_characters(line: &str, opts: &CutOptions) -> String {
    let chars: Vec<char> = line.chars().collect();
    let selected = select_positions(chars.len(), &opts.ranges, opts.complement);
    let out_delim = opts.output_delimiter.as_deref().unwrap_or("");

    let mut parts: Vec<String> = Vec::new();
    let mut current_run = String::new();
    let mut last_pos: Option<usize> = None;

    for pos in &selected {
        let idx = pos - 1;
        if idx < chars.len() {
            if let Some(lp) = last_pos {
                if *pos != lp + 1 && !out_delim.is_empty() {
                    parts.push(current_run.clone());
                    current_run.clear();
                }
            }
            current_run.push(chars[idx]);
            last_pos = Some(*pos);
        }
    }
    if !current_run.is_empty() {
        parts.push(current_run);
    }

    if out_delim.is_empty() || parts.len() <= 1 {
        parts.join("")
    } else {
        parts.join(out_delim)
    }
}

/// Cut bytes from a line based on byte ranges.
///
/// This operates on raw bytes. Multi-byte UTF-8 characters may be
/// split, which matches the behavior of GNU cut -b.
pub fn cut_bytes(line: &str, opts: &CutOptions) -> String {
    let bytes = line.as_bytes();
    let selected = select_positions(bytes.len(), &opts.ranges, opts.complement);

    let result_bytes: Vec<u8> = selected
        .iter()
        .filter_map(|pos| {
            let idx = pos - 1;
            if idx < bytes.len() {
                Some(bytes[idx])
            } else {
                None
            }
        })
        .collect();

    String::from_utf8_lossy(&result_bytes).into_owned()
}

/// Cut fields from a line based on field ranges.
///
/// Fields are delimited by a single delimiter character (default: TAB).
/// If -s is set and the line doesn't contain the delimiter, it is
/// suppressed (returns None).
///
/// # Example
///
/// ```text
///     cut_fields("a\tb\tc", ranges=[Single(2)], delim='\t')
///     → Some("b")
/// ```
pub fn cut_fields(line: &str, opts: &CutOptions) -> Option<String> {
    // --- Check for delimiter ---
    // If -s is set and the line has no delimiter, suppress it.
    if opts.only_delimited && !line.contains(opts.delimiter) {
        return None;
    }

    // If the line has no delimiter and -s is not set, print the whole line.
    if !line.contains(opts.delimiter) {
        return Some(line.to_string());
    }

    let fields: Vec<&str> = line.split(opts.delimiter).collect();
    let selected = select_positions(fields.len(), &opts.ranges, opts.complement);

    let default_delim = opts.delimiter.to_string();
    let out_delim = opts.output_delimiter
        .as_deref()
        .unwrap_or(&default_delim);

    let result: Vec<&str> = selected
        .iter()
        .filter_map(|pos| {
            let idx = pos - 1;
            if idx < fields.len() {
                Some(fields[idx])
            } else {
                None
            }
        })
        .collect();

    Some(result.join(out_delim))
}

/// Given a total count and a list of ranges, expand them into a sorted
/// list of unique 1-based positions.
///
/// If `complement` is true, returns all positions NOT in the expanded set.
fn select_positions(total: usize, ranges: &[Range], complement: bool) -> Vec<usize> {
    let mut positions = std::collections::BTreeSet::new();

    for range in ranges {
        match range {
            Range::Single(n) => {
                if *n <= total {
                    positions.insert(*n);
                }
            }
            Range::Closed(start, end) => {
                let s = *start;
                let e = (*end).min(total);
                for pos in s..=e {
                    positions.insert(pos);
                }
            }
            Range::From(start) => {
                for pos in *start..=total {
                    positions.insert(pos);
                }
            }
            Range::To(end) => {
                let e = (*end).min(total);
                for pos in 1..=e {
                    positions.insert(pos);
                }
            }
        }
    }

    if complement {
        (1..=total).filter(|p| !positions.contains(p)).collect()
    } else {
        positions.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Range parsing tests ---

    #[test]
    fn parse_single() {
        let ranges = parse_range_list("3").unwrap();
        assert_eq!(ranges, vec![Range::Single(3)]);
    }

    #[test]
    fn parse_closed_range() {
        let ranges = parse_range_list("2-5").unwrap();
        assert_eq!(ranges, vec![Range::Closed(2, 5)]);
    }

    #[test]
    fn parse_from_range() {
        let ranges = parse_range_list("3-").unwrap();
        assert_eq!(ranges, vec![Range::From(3)]);
    }

    #[test]
    fn parse_to_range() {
        let ranges = parse_range_list("-4").unwrap();
        assert_eq!(ranges, vec![Range::To(4)]);
    }

    #[test]
    fn parse_multiple() {
        let ranges = parse_range_list("1,3-5,7-").unwrap();
        assert_eq!(ranges, vec![
            Range::Single(1),
            Range::Closed(3, 5),
            Range::From(7),
        ]);
    }

    #[test]
    fn parse_zero_is_error() {
        assert!(parse_range_list("0").is_err());
    }

    // --- Character cutting tests ---

    #[test]
    fn cut_chars_single() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1)],
            ..Default::default()
        };
        assert_eq!(cut_characters("hello", &opts), "h");
    }

    #[test]
    fn cut_chars_range() {
        let opts = CutOptions {
            ranges: vec![Range::Closed(1, 5)],
            ..Default::default()
        };
        assert_eq!(cut_characters("hello world", &opts), "hello");
    }

    #[test]
    fn cut_chars_from() {
        let opts = CutOptions {
            ranges: vec![Range::From(7)],
            ..Default::default()
        };
        assert_eq!(cut_characters("hello world", &opts), "world");
    }

    // --- Field cutting tests ---

    #[test]
    fn cut_fields_tab_delimited() {
        let opts = CutOptions {
            ranges: vec![Range::Single(2)],
            ..Default::default()
        };
        assert_eq!(cut_fields("a\tb\tc", &opts), Some("b".to_string()));
    }

    #[test]
    fn cut_fields_custom_delimiter() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1), Range::Single(3)],
            delimiter: ':',
            ..Default::default()
        };
        assert_eq!(cut_fields("a:b:c", &opts), Some("a:c".to_string()));
    }

    #[test]
    fn cut_fields_only_delimited() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1)],
            only_delimited: true,
            ..Default::default()
        };
        assert_eq!(cut_fields("no tabs here", &opts), None);
    }

    #[test]
    fn cut_fields_no_delimiter_no_suppress() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1)],
            ..Default::default()
        };
        assert_eq!(
            cut_fields("no tabs here", &opts),
            Some("no tabs here".to_string())
        );
    }

    // --- Complement tests ---

    #[test]
    fn complement_chars() {
        let opts = CutOptions {
            ranges: vec![Range::Single(1), Range::Single(3)],
            complement: true,
            ..Default::default()
        };
        // "abcde" → positions 1,3 selected → complement is 2,4,5
        assert_eq!(cut_characters("abcde", &opts), "bde");
    }

    // --- Byte cutting tests ---

    #[test]
    fn cut_bytes_range() {
        let opts = CutOptions {
            ranges: vec![Range::Closed(1, 5)],
            ..Default::default()
        };
        assert_eq!(cut_bytes("hello world", &opts), "hello");
    }

    // --- Select positions ---

    #[test]
    fn select_positions_basic() {
        let result = select_positions(10, &[Range::Closed(3, 5)], false);
        assert_eq!(result, vec![3, 4, 5]);
    }

    #[test]
    fn select_positions_complement() {
        let result = select_positions(5, &[Range::Closed(2, 4)], true);
        assert_eq!(result, vec![1, 5]);
    }
}
