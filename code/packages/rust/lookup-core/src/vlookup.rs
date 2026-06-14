//! VLOOKUP / HLOOKUP — the two original "lookup in a table by key" functions.
//!
//! Both functions search a row or column for a probe and return a value from
//! a parallel column or row.  They differ only in axis orientation, so the
//! implementation factors through a shared "search a 1-D key column" helper.
//!
//! # Excel parity notes
//!
//! - Excel's `range_lookup` parameter defaults to `TRUE` (approximate
//!   match).  We make `range_lookup` a *required* parameter at the Rust
//!   surface because silent approximate-match is a famous source of
//!   spreadsheet bugs.  The dispatcher that wraps this crate is responsible
//!   for filling in the default when emulating Excel's call shape.
//! - Approximate match requires the key column to be sorted ascending.  We
//!   trust the caller and binary-search; this matches Excel's documented
//!   contract.
//! - NA in the probe → NA result (returned as `Ok(LookupValue::na())`).
//! - NA cells in the key column are *skipped* — they never match.

use crate::{
    cmp_excel, equal_excel, is_na_real, one_based_to_zero, LookupError, LookupResult, LookupValue,
};

/// Excel `VLOOKUP(lookup_value, table, col_index, range_lookup)`.
///
/// - `table`: row-major 2-D array.  The *first* column is the key column.
/// - `col_index`: 1-based column index into `table`.
/// - `range_lookup = true`: approximate match (sorted ascending).
/// - `range_lookup = false`: exact match (case-insensitive ASCII for text).
pub fn vlookup(
    lookup_value: &LookupValue,
    table: &[Vec<LookupValue>],
    col_index: i64,
    range_lookup: bool,
) -> LookupResult<LookupValue> {
    if table.is_empty() {
        return Err(LookupError::NotFound { function: "VLOOKUP" });
    }
    // NA probe propagates regardless of match mode.
    if lookup_value.is_na() {
        return Ok(LookupValue::na());
    }
    let n_cols = table[0].len();
    let col_zero = one_based_to_zero("VLOOKUP", col_index, n_cols)?;
    let keys: Vec<&LookupValue> = table.iter().map(|row| &row[0]).collect();

    let row = search_key_column("VLOOKUP", lookup_value, &keys, range_lookup)?;
    Ok(table[row][col_zero].clone())
}

/// Excel `HLOOKUP(lookup_value, table, row_index, range_lookup)`.
///
/// Mirrors [`vlookup`], but the *first row* of `table` is the key row and we
/// return from column-aligned cells in `row_index`.
pub fn hlookup(
    lookup_value: &LookupValue,
    table: &[Vec<LookupValue>],
    row_index: i64,
    range_lookup: bool,
) -> LookupResult<LookupValue> {
    if table.is_empty() || table[0].is_empty() {
        return Err(LookupError::NotFound { function: "HLOOKUP" });
    }
    if lookup_value.is_na() {
        return Ok(LookupValue::na());
    }
    let n_rows = table.len();
    let row_zero = one_based_to_zero("HLOOKUP", row_index, n_rows)?;
    let keys: Vec<&LookupValue> = table[0].iter().collect();

    let col = search_key_column("HLOOKUP", lookup_value, &keys, range_lookup)?;
    Ok(table[row_zero][col].clone())
}

/// Shared key-column search.  Skips NA entries (Excel semantics) and either
/// performs exact equality (`range_lookup=false`) or an approximate-match
/// binary search (`range_lookup=true`).
fn search_key_column(
    function: &'static str,
    probe: &LookupValue,
    keys: &[&LookupValue],
    range_lookup: bool,
) -> LookupResult<usize> {
    if !range_lookup {
        // Linear exact-match scan.  We must use a linear scan rather than
        // binary search because exact-match VLOOKUP does NOT require the key
        // column to be sorted.
        for (i, key) in keys.iter().enumerate() {
            if key_is_na(key) {
                continue;
            }
            if equal_excel(probe, key) {
                return Ok(i);
            }
        }
        return Err(LookupError::NotFound { function });
    }

    // Approximate match: binary search for the *largest* key ≤ probe.
    //
    // The key column is assumed sorted ascending with NAs skipped.  We work
    // on a compacted (no-NA) view to keep the binary search invariant clean,
    // then map back to original indices at the end.
    let compact: Vec<(usize, &LookupValue)> = keys
        .iter()
        .enumerate()
        .filter(|(_, k)| !key_is_na(k))
        .map(|(i, k)| (i, *k))
        .collect();

    if compact.is_empty() {
        return Err(LookupError::NotFound { function });
    }

    // Reject if probe is smaller than the smallest key — Excel returns #N/A
    // in this case rather than returning the first row.
    if cmp_excel(function, probe, compact[0].1)?.is_lt() {
        return Err(LookupError::NotFound { function });
    }

    let mut lo = 0usize;
    let mut hi = compact.len();
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        // We want the largest index with key ≤ probe.  If keys[mid] > probe,
        // shrink the high end; otherwise the answer is at mid or further.
        match cmp_excel(function, compact[mid].1, probe)? {
            std::cmp::Ordering::Greater => hi = mid,
            _ => lo = mid,
        }
    }
    Ok(compact[lo].0)
}

fn key_is_na(v: &LookupValue) -> bool {
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
    fn vlookup_exact_hit_returns_aligned_value() {
        // Two-column lookup table: (fruit, price)
        let table = vec![
            vec![t("apple"), n(1.0)],
            vec![t("banana"), n(2.0)],
            vec![t("cherry"), n(3.0)],
        ];
        let r = vlookup(&t("banana"), &table, 2, false).unwrap();
        assert_eq!(r, n(2.0));
    }

    #[test]
    fn vlookup_exact_miss_is_not_found() {
        let table = vec![vec![t("a"), n(1.0)], vec![t("b"), n(2.0)]];
        let err = vlookup(&t("z"), &table, 2, false).unwrap_err();
        assert!(matches!(err, LookupError::NotFound { function: "VLOOKUP" }));
    }

    #[test]
    fn vlookup_text_is_case_insensitive() {
        let table = vec![vec![t("Apple"), n(1.0)]];
        let r = vlookup(&t("apple"), &table, 2, false).unwrap();
        assert_eq!(r, n(1.0));
    }

    #[test]
    fn vlookup_col_index_out_of_range() {
        let table = vec![vec![t("a"), n(1.0)]];
        let err = vlookup(&t("a"), &table, 5, false).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn vlookup_approximate_picks_largest_le_probe() {
        // Bracketed tax table.
        let table = vec![
            vec![n(0.0), t("0%")],
            vec![n(10.0), t("10%")],
            vec![n(20.0), t("20%")],
            vec![n(50.0), t("50%")],
        ];
        assert_eq!(vlookup(&n(15.0), &table, 2, true).unwrap(), t("10%"));
        assert_eq!(vlookup(&n(50.0), &table, 2, true).unwrap(), t("50%"));
        assert_eq!(vlookup(&n(0.0), &table, 2, true).unwrap(), t("0%"));
        assert_eq!(vlookup(&n(1000.0), &table, 2, true).unwrap(), t("50%"));
    }

    #[test]
    fn vlookup_approximate_below_min_is_not_found() {
        let table = vec![vec![n(10.0), t("10%")], vec![n(20.0), t("20%")]];
        let err = vlookup(&n(5.0), &table, 2, true).unwrap_err();
        assert!(matches!(err, LookupError::NotFound { .. }));
    }

    #[test]
    fn vlookup_skips_na_keys() {
        let table = vec![
            vec![LookupValue::na(), t("ghost")],
            vec![t("apple"), t("a")],
        ];
        let r = vlookup(&t("apple"), &table, 2, false).unwrap();
        assert_eq!(r, t("a"));
    }

    #[test]
    fn vlookup_na_probe_propagates() {
        let table = vec![vec![n(1.0), n(10.0)]];
        let r = vlookup(&LookupValue::na(), &table, 2, false).unwrap();
        assert!(r.is_na());
    }

    #[test]
    fn vlookup_type_mismatch_in_approx_is_error() {
        // Approximate match on a text probe against numeric keys.
        let table = vec![vec![n(1.0), t("a")], vec![n(2.0), t("b")]];
        let err = vlookup(&t("x"), &table, 2, true).unwrap_err();
        assert!(matches!(err, LookupError::TypeMismatch { .. }));
    }

    #[test]
    fn hlookup_exact_hit_returns_aligned_value() {
        // First row is keys, second row is values.
        let table = vec![vec![t("a"), t("b"), t("c")], vec![n(1.0), n(2.0), n(3.0)]];
        assert_eq!(hlookup(&t("b"), &table, 2, false).unwrap(), n(2.0));
    }

    #[test]
    fn hlookup_approximate() {
        let table = vec![vec![n(0.0), n(10.0), n(20.0)], vec![t("low"), t("mid"), t("hi")]];
        assert_eq!(hlookup(&n(15.0), &table, 2, true).unwrap(), t("mid"));
        assert_eq!(hlookup(&n(100.0), &table, 2, true).unwrap(), t("hi"));
    }

    #[test]
    fn hlookup_row_index_out_of_range() {
        let table = vec![vec![t("a")]];
        let err = hlookup(&t("a"), &table, 99, false).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn hlookup_empty_table_not_found() {
        let table: Vec<Vec<LookupValue>> = vec![];
        let err = hlookup(&t("a"), &table, 1, false).unwrap_err();
        assert!(matches!(err, LookupError::NotFound { .. }));
    }

    #[test]
    fn vlookup_approximate_edge_cases_min_max_between() {
        // Verify binary-search behaviour at exact min, exact max, and a
        // value between two existing keys.
        let table = vec![
            vec![n(1.0), t("a")],
            vec![n(3.0), t("b")],
            vec![n(5.0), t("c")],
            vec![n(7.0), t("d")],
            vec![n(9.0), t("e")],
        ];
        assert_eq!(vlookup(&n(1.0), &table, 2, true).unwrap(), t("a")); // exact min
        assert_eq!(vlookup(&n(9.0), &table, 2, true).unwrap(), t("e")); // exact max
        assert_eq!(vlookup(&n(4.0), &table, 2, true).unwrap(), t("b")); // between
        assert_eq!(vlookup(&n(6.0), &table, 2, true).unwrap(), t("c"));
        assert_eq!(vlookup(&n(8.0), &table, 2, true).unwrap(), t("d"));
    }
}
