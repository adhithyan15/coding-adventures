//! INDEX and MATCH — the building blocks behind every modern lookup
//! formula.  INDEX returns a value (or whole row/column) from an array by
//! coordinate; MATCH returns the position of a probe within a 1-D array.
//!
//! Together they replace VLOOKUP for arbitrary-direction lookups: the
//! canonical idiom is `INDEX(return_col, MATCH(probe, key_col, 0))`.

use crate::{cmp_excel, equal_excel, is_na_real, one_based_to_zero, LookupError, LookupResult, LookupValue};

/// MATCH variants — Excel's `match_type` integer rewritten as an enum so
/// callers can't accidentally pass an unsupported value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// `match_type = 0`: exact match, no sort assumption (linear scan).
    Exact,
    /// `match_type = 1`: largest value `<=` probe; lookup array must be
    /// sorted ascending.
    LessOrEqual,
    /// `match_type = -1`: smallest value `>=` probe; lookup array must be
    /// sorted descending.
    GreaterOrEqual,
}

impl MatchType {
    /// Build a `MatchType` from Excel's integer code, surfacing bad values
    /// as `LookupError::BadParameter`.
    pub fn from_excel_code(code: i64) -> LookupResult<Self> {
        match code {
            0 => Ok(MatchType::Exact),
            1 => Ok(MatchType::LessOrEqual),
            -1 => Ok(MatchType::GreaterOrEqual),
            _ => Err(LookupError::BadParameter {
                name: "match_type",
                value: code.to_string(),
            }),
        }
    }
}

/// `INDEX(array, n)` over a 1-D vector.  `n` is 1-based.
pub fn index_1d(array: &[LookupValue], n: i64) -> LookupResult<LookupValue> {
    let zero = one_based_to_zero("INDEX", n, array.len())?;
    Ok(array[zero].clone())
}

/// `INDEX(array, row, col)` over a 2-D row-major matrix.
///
/// Special cases (Excel parity):
/// - `row = 0`  → return the whole column `col` as a vector.
/// - `col = 0`  → return the whole row `row` as a vector.
/// - both zero  → error (Excel returns the entire array; we surface as
///   `BadParameter` since this crate's return type is a single value or a
///   single vector, not a matrix).
///
/// The "whole row" / "whole column" results are returned as a separate
/// [`IndexResult`] discriminant so the caller can tell them apart from a
/// scalar.
pub fn index_2d(array: &[Vec<LookupValue>], row: i64, col: i64) -> LookupResult<IndexResult> {
    if array.is_empty() {
        return Err(LookupError::OutOfRange {
            function: "INDEX",
            index: row,
            max: 0,
        });
    }
    let n_rows = array.len();
    let n_cols = array[0].len();

    match (row, col) {
        (0, 0) => Err(LookupError::BadParameter {
            name: "row,col",
            value: "0,0 (whole-array result not supported in scalar API)".to_string(),
        }),
        (0, c) => {
            let cz = one_based_to_zero("INDEX", c, n_cols)?;
            let column: Vec<LookupValue> = array.iter().map(|r| r[cz].clone()).collect();
            Ok(IndexResult::Vector(column))
        }
        (r, 0) => {
            let rz = one_based_to_zero("INDEX", r, n_rows)?;
            Ok(IndexResult::Vector(array[rz].clone()))
        }
        (r, c) => {
            let rz = one_based_to_zero("INDEX", r, n_rows)?;
            let cz = one_based_to_zero("INDEX", c, n_cols)?;
            Ok(IndexResult::Scalar(array[rz][cz].clone()))
        }
    }
}

/// Result of [`index_2d`] — either a single value or a whole row/column.
#[derive(Debug, Clone, PartialEq)]
pub enum IndexResult {
    Scalar(LookupValue),
    Vector(Vec<LookupValue>),
}

/// `MATCH(lookup_value, lookup_array, match_type)`.  Returns a 1-based
/// position.  NA in the probe propagates as `#N/A`; NA in the array is
/// skipped.
pub fn r#match(
    lookup_value: &LookupValue,
    lookup_array: &[LookupValue],
    match_type: MatchType,
) -> LookupResult<i64> {
    if lookup_value.is_na() {
        return Err(LookupError::NotFound { function: "MATCH" });
    }
    match match_type {
        MatchType::Exact => match_exact(lookup_value, lookup_array),
        MatchType::LessOrEqual => match_le_sorted_asc(lookup_value, lookup_array),
        MatchType::GreaterOrEqual => match_ge_sorted_desc(lookup_value, lookup_array),
    }
}

fn match_exact(probe: &LookupValue, array: &[LookupValue]) -> LookupResult<i64> {
    for (i, value) in array.iter().enumerate() {
        if matches!(value, LookupValue::Number(n) if is_na_real(*n)) {
            continue;
        }
        if equal_excel(probe, value) {
            return Ok((i + 1) as i64);
        }
    }
    Err(LookupError::NotFound { function: "MATCH" })
}

fn match_le_sorted_asc(probe: &LookupValue, array: &[LookupValue]) -> LookupResult<i64> {
    // Largest value <= probe.  Array assumed sorted ascending.
    let compact: Vec<(usize, &LookupValue)> = array
        .iter()
        .enumerate()
        .filter(|(_, v)| !is_na_value(v))
        .collect();
    if compact.is_empty() {
        return Err(LookupError::NotFound { function: "MATCH" });
    }
    if cmp_excel("MATCH", probe, compact[0].1)?.is_lt() {
        return Err(LookupError::NotFound { function: "MATCH" });
    }
    let mut lo = 0usize;
    let mut hi = compact.len();
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        match cmp_excel("MATCH", compact[mid].1, probe)? {
            std::cmp::Ordering::Greater => hi = mid,
            _ => lo = mid,
        }
    }
    Ok((compact[lo].0 + 1) as i64)
}

fn match_ge_sorted_desc(probe: &LookupValue, array: &[LookupValue]) -> LookupResult<i64> {
    // Smallest value >= probe.  Array assumed sorted descending.
    let compact: Vec<(usize, &LookupValue)> = array
        .iter()
        .enumerate()
        .filter(|(_, v)| !is_na_value(v))
        .collect();
    if compact.is_empty() {
        return Err(LookupError::NotFound { function: "MATCH" });
    }
    // Descending: first element is the max.  If probe > first element,
    // nothing in the array is >= probe → #N/A.
    if cmp_excel("MATCH", probe, compact[0].1)?.is_gt() {
        return Err(LookupError::NotFound { function: "MATCH" });
    }
    // Linear scan from start: pick the LAST index whose value is still
    // >= probe.  Sticking to a linear scan keeps the logic simple; the
    // typical lookup array is small enough that this is not a hotspot.
    let mut answer: Option<usize> = None;
    for (i, v) in &compact {
        if cmp_excel("MATCH", v, probe)?.is_ge() {
            answer = Some(*i);
        } else {
            break;
        }
    }
    answer
        .map(|i| (i + 1) as i64)
        .ok_or(LookupError::NotFound { function: "MATCH" })
}

fn is_na_value(v: &LookupValue) -> bool {
    matches!(v, LookupValue::Number(n) if is_na_real(*n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> LookupValue {
        LookupValue::Text(s.into())
    }
    fn n(x: f64) -> LookupValue {
        LookupValue::Number(x)
    }

    #[test]
    fn index_1d_basic() {
        let arr = vec![t("a"), t("b"), t("c")];
        assert_eq!(index_1d(&arr, 1).unwrap(), t("a"));
        assert_eq!(index_1d(&arr, 3).unwrap(), t("c"));
    }

    #[test]
    fn index_1d_out_of_range() {
        let arr = vec![t("a")];
        assert!(matches!(
            index_1d(&arr, 5).unwrap_err(),
            LookupError::OutOfRange { .. }
        ));
        assert!(matches!(
            index_1d(&arr, 0).unwrap_err(),
            LookupError::OutOfRange { .. }
        ));
    }

    #[test]
    fn index_2d_scalar_pick() {
        let m = vec![vec![n(1.0), n(2.0)], vec![n(3.0), n(4.0)]];
        match index_2d(&m, 2, 1).unwrap() {
            IndexResult::Scalar(v) => assert_eq!(v, n(3.0)),
            _ => panic!("expected scalar"),
        }
    }

    #[test]
    fn index_2d_whole_row() {
        let m = vec![vec![n(1.0), n(2.0)], vec![n(3.0), n(4.0)]];
        match index_2d(&m, 1, 0).unwrap() {
            IndexResult::Vector(v) => assert_eq!(v, vec![n(1.0), n(2.0)]),
            _ => panic!(),
        }
    }

    #[test]
    fn index_2d_whole_column() {
        let m = vec![vec![n(1.0), n(2.0)], vec![n(3.0), n(4.0)]];
        match index_2d(&m, 0, 2).unwrap() {
            IndexResult::Vector(v) => assert_eq!(v, vec![n(2.0), n(4.0)]),
            _ => panic!(),
        }
    }

    #[test]
    fn index_2d_both_zero_is_bad_param() {
        let m = vec![vec![n(1.0)]];
        assert!(matches!(
            index_2d(&m, 0, 0).unwrap_err(),
            LookupError::BadParameter { .. }
        ));
    }

    #[test]
    fn match_exact_returns_one_based_position() {
        let arr = vec![t("a"), t("b"), t("c")];
        assert_eq!(r#match(&t("b"), &arr, MatchType::Exact).unwrap(), 2);
    }

    #[test]
    fn match_exact_miss_is_not_found() {
        let arr = vec![t("a"), t("b")];
        assert!(matches!(
            r#match(&t("z"), &arr, MatchType::Exact).unwrap_err(),
            LookupError::NotFound { .. }
        ));
    }

    #[test]
    fn match_le_sorted_asc_picks_largest_le() {
        let arr = vec![n(1.0), n(3.0), n(5.0), n(7.0)];
        assert_eq!(r#match(&n(4.0), &arr, MatchType::LessOrEqual).unwrap(), 2);
        assert_eq!(r#match(&n(7.0), &arr, MatchType::LessOrEqual).unwrap(), 4);
        assert_eq!(r#match(&n(1.0), &arr, MatchType::LessOrEqual).unwrap(), 1);
    }

    #[test]
    fn match_le_below_min_not_found() {
        let arr = vec![n(10.0), n(20.0)];
        assert!(matches!(
            r#match(&n(5.0), &arr, MatchType::LessOrEqual).unwrap_err(),
            LookupError::NotFound { .. }
        ));
    }

    #[test]
    fn match_ge_sorted_desc_picks_smallest_ge() {
        let arr = vec![n(9.0), n(7.0), n(5.0), n(3.0), n(1.0)];
        assert_eq!(
            r#match(&n(4.0), &arr, MatchType::GreaterOrEqual).unwrap(),
            3
        );
        assert_eq!(
            r#match(&n(9.0), &arr, MatchType::GreaterOrEqual).unwrap(),
            1
        );
        assert_eq!(
            r#match(&n(1.0), &arr, MatchType::GreaterOrEqual).unwrap(),
            5
        );
    }

    #[test]
    fn match_ge_above_max_not_found() {
        let arr = vec![n(9.0), n(7.0)];
        assert!(matches!(
            r#match(&n(100.0), &arr, MatchType::GreaterOrEqual).unwrap_err(),
            LookupError::NotFound { .. }
        ));
    }

    #[test]
    fn match_excel_code_parser_round_trips() {
        assert_eq!(MatchType::from_excel_code(0).unwrap(), MatchType::Exact);
        assert_eq!(MatchType::from_excel_code(1).unwrap(), MatchType::LessOrEqual);
        assert_eq!(
            MatchType::from_excel_code(-1).unwrap(),
            MatchType::GreaterOrEqual
        );
        assert!(matches!(
            MatchType::from_excel_code(7).unwrap_err(),
            LookupError::BadParameter { .. }
        ));
    }

    #[test]
    fn match_skips_na_array_entries() {
        let arr = vec![LookupValue::na(), t("a"), t("b")];
        assert_eq!(r#match(&t("a"), &arr, MatchType::Exact).unwrap(), 2);
    }

    #[test]
    fn match_na_probe_is_not_found() {
        let arr = vec![t("a")];
        assert!(matches!(
            r#match(&LookupValue::na(), &arr, MatchType::Exact).unwrap_err(),
            LookupError::NotFound { .. }
        ));
    }

    #[test]
    fn match_mixed_types_in_array_with_exact() {
        // Mixed numbers and text — exact match still works because we only
        // compare same-type pairs and return the first equal entry.
        let arr = vec![t("apple"), n(1.0), t("banana"), n(2.0)];
        assert_eq!(r#match(&n(2.0), &arr, MatchType::Exact).unwrap(), 4);
        assert_eq!(r#match(&t("banana"), &arr, MatchType::Exact).unwrap(), 3);
    }
}
