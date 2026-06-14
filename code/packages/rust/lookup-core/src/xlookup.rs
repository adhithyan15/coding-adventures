//! XLOOKUP / XMATCH — Excel's modern lookup primitives.  Compared to
//! VLOOKUP, XLOOKUP:
//!
//! - Defaults to **exact match** (no silent approximate-match foot-gun).
//! - Returns from an independent return array, so the lookup column does
//!   not have to be left of the result column.
//! - Lets callers supply a fallback (`if_not_found`) instead of an error.
//! - Supports wildcard and binary-search modes.

use crate::{
    cmp_excel, equal_excel, is_na_real, matches_wildcard, LookupError, LookupResult, LookupValue,
};

/// `match_mode` for XLOOKUP / XMATCH.  Matches Excel's integer codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XMatchMode {
    /// `0`: exact match (default in Excel).
    Exact,
    /// `-1`: exact, or next smaller item.
    ExactOrNextSmaller,
    /// `1`: exact, or next larger item.
    ExactOrNextLarger,
    /// `2`: wildcard match (`*` and `?` with `~` as escape).
    Wildcard,
}

impl XMatchMode {
    pub fn from_excel_code(code: i64) -> LookupResult<Self> {
        match code {
            0 => Ok(XMatchMode::Exact),
            -1 => Ok(XMatchMode::ExactOrNextSmaller),
            1 => Ok(XMatchMode::ExactOrNextLarger),
            2 => Ok(XMatchMode::Wildcard),
            _ => Err(LookupError::BadParameter {
                name: "match_mode",
                value: code.to_string(),
            }),
        }
    }
}

/// `search_mode` for XLOOKUP / XMATCH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XSearchMode {
    /// `1`: scan from first to last (default).
    FirstToLast,
    /// `-1`: scan from last to first.
    LastToFirst,
    /// `2`: binary search assuming ascending order.
    BinarySortedAsc,
    /// `-2`: binary search assuming descending order.
    BinarySortedDesc,
}

impl XSearchMode {
    pub fn from_excel_code(code: i64) -> LookupResult<Self> {
        match code {
            1 => Ok(XSearchMode::FirstToLast),
            -1 => Ok(XSearchMode::LastToFirst),
            2 => Ok(XSearchMode::BinarySortedAsc),
            -2 => Ok(XSearchMode::BinarySortedDesc),
            _ => Err(LookupError::BadParameter {
                name: "search_mode",
                value: code.to_string(),
            }),
        }
    }
}

/// `XLOOKUP(lookup_value, lookup_array, return_array, if_not_found,
/// match_mode, search_mode)`.
///
/// `if_not_found` is `Some(value)` to suppress the `NotFound` error and
/// return `value` instead — exactly Excel's behaviour when the optional
/// fallback argument is supplied.
pub fn xlookup(
    lookup_value: &LookupValue,
    lookup_array: &[LookupValue],
    return_array: &[LookupValue],
    if_not_found: Option<LookupValue>,
    match_mode: XMatchMode,
    search_mode: XSearchMode,
) -> LookupResult<LookupValue> {
    if lookup_array.len() != return_array.len() {
        return Err(LookupError::ShapeMismatch {
            expected: format!("return_array.len = {}", lookup_array.len()),
            found: format!("{}", return_array.len()),
        });
    }
    if lookup_value.is_na() {
        return Ok(LookupValue::na());
    }
    match xmatch(lookup_value, lookup_array, match_mode, search_mode) {
        Ok(pos) => {
            // xmatch returns 1-based; convert to 0-based to index return_array.
            Ok(return_array[(pos - 1) as usize].clone())
        }
        Err(LookupError::NotFound { .. }) => match if_not_found {
            Some(v) => Ok(v),
            None => Err(LookupError::NotFound { function: "XLOOKUP" }),
        },
        Err(e) => Err(e),
    }
}

/// `XMATCH(lookup_value, lookup_array, match_mode, search_mode)`.
/// Returns a 1-based position.
pub fn xmatch(
    lookup_value: &LookupValue,
    lookup_array: &[LookupValue],
    match_mode: XMatchMode,
    search_mode: XSearchMode,
) -> LookupResult<i64> {
    if lookup_value.is_na() {
        return Err(LookupError::NotFound { function: "XMATCH" });
    }
    if lookup_array.is_empty() {
        return Err(LookupError::NotFound { function: "XMATCH" });
    }

    // Build an iteration order based on `search_mode`.  Binary search modes
    // get their own fast path below; the linear modes share a generic
    // "iterate + test" loop.
    match search_mode {
        XSearchMode::FirstToLast => linear_scan(lookup_value, lookup_array, match_mode, false),
        XSearchMode::LastToFirst => linear_scan(lookup_value, lookup_array, match_mode, true),
        XSearchMode::BinarySortedAsc => {
            binary_search_lookup(lookup_value, lookup_array, match_mode, true)
        }
        XSearchMode::BinarySortedDesc => {
            binary_search_lookup(lookup_value, lookup_array, match_mode, false)
        }
    }
}

fn linear_scan(
    probe: &LookupValue,
    array: &[LookupValue],
    mode: XMatchMode,
    reverse: bool,
) -> LookupResult<i64> {
    // For ExactOrNextSmaller / ExactOrNextLarger we need to track the best
    // candidate so far in addition to a possible exact hit, because the
    // first exact match wins immediately but otherwise we have to scan all
    // elements to find the closest one in the requested direction.
    let n = array.len();
    let order: Box<dyn Iterator<Item = usize>> = if reverse {
        Box::new((0..n).rev())
    } else {
        Box::new(0..n)
    };

    let mut best: Option<(usize, &LookupValue)> = None;
    for i in order {
        let value = &array[i];
        if matches!(value, LookupValue::Number(n) if is_na_real(*n)) {
            continue;
        }
        match mode {
            XMatchMode::Exact => {
                if equal_excel(probe, value) {
                    return Ok((i + 1) as i64);
                }
            }
            XMatchMode::Wildcard => {
                if matches_wildcard(probe, value) {
                    return Ok((i + 1) as i64);
                }
            }
            XMatchMode::ExactOrNextSmaller => {
                if equal_excel(probe, value) {
                    return Ok((i + 1) as i64);
                }
                let ord = cmp_excel("XMATCH", value, probe)?;
                if ord.is_lt() {
                    // Track the *largest* such value.
                    if let Some((_, b)) = best {
                        if cmp_excel("XMATCH", value, b)?.is_gt() {
                            best = Some((i, value));
                        }
                    } else {
                        best = Some((i, value));
                    }
                }
            }
            XMatchMode::ExactOrNextLarger => {
                if equal_excel(probe, value) {
                    return Ok((i + 1) as i64);
                }
                let ord = cmp_excel("XMATCH", value, probe)?;
                if ord.is_gt() {
                    // Track the *smallest* such value.
                    if let Some((_, b)) = best {
                        if cmp_excel("XMATCH", value, b)?.is_lt() {
                            best = Some((i, value));
                        }
                    } else {
                        best = Some((i, value));
                    }
                }
            }
        }
    }

    if let Some((i, _)) = best {
        return Ok((i + 1) as i64);
    }
    Err(LookupError::NotFound { function: "XMATCH" })
}

fn binary_search_lookup(
    probe: &LookupValue,
    array: &[LookupValue],
    mode: XMatchMode,
    ascending: bool,
) -> LookupResult<i64> {
    // Binary search modes assume no NAs in the array; Excel does too — NA
    // would break the sort order.  We still skip NAs defensively, but doing
    // so degrades to a compacted view rather than tripping over them.
    let compact: Vec<(usize, &LookupValue)> = array
        .iter()
        .enumerate()
        .filter(|(_, v)| !matches!(v, LookupValue::Number(n) if is_na_real(*n)))
        .collect();
    if compact.is_empty() {
        return Err(LookupError::NotFound { function: "XMATCH" });
    }

    // Classic binary search for equality.  Falls back to the directional
    // "next-smaller"/"next-larger" rules if no exact hit.
    let mut lo = 0usize;
    let mut hi = compact.len();
    let cmp = |a: &LookupValue, b: &LookupValue| cmp_excel("XMATCH", a, b);

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let ord = cmp(compact[mid].1, probe)?;
        match (ord, ascending) {
            (std::cmp::Ordering::Equal, _) => return Ok((compact[mid].0 + 1) as i64),
            (std::cmp::Ordering::Less, true) | (std::cmp::Ordering::Greater, false) => {
                lo = mid + 1;
            }
            _ => hi = mid,
        }
    }

    // Not found exactly.  Apply directional fallback.  After the loop, `lo`
    // is the insertion point in the compacted, *iteration-direction-sorted*
    // view: everything before `lo` is "less-than-probe-in-iteration-order"
    // and everything from `lo` onward is "greater-than-probe-in-iteration-order".
    match mode {
        XMatchMode::Exact | XMatchMode::Wildcard => {
            Err(LookupError::NotFound { function: "XMATCH" })
        }
        XMatchMode::ExactOrNextSmaller => {
            if ascending {
                // The element right before `lo` is the largest value strictly
                // less than the probe.
                if lo == 0 {
                    return Err(LookupError::NotFound { function: "XMATCH" });
                }
                Ok((compact[lo - 1].0 + 1) as i64)
            } else {
                // In descending order, "less than probe" sits at index `lo`
                // (everything past lo is smaller-than-probe in true value).
                if lo >= compact.len() {
                    return Err(LookupError::NotFound { function: "XMATCH" });
                }
                Ok((compact[lo].0 + 1) as i64)
            }
        }
        XMatchMode::ExactOrNextLarger => {
            if ascending {
                if lo >= compact.len() {
                    return Err(LookupError::NotFound { function: "XMATCH" });
                }
                Ok((compact[lo].0 + 1) as i64)
            } else {
                if lo == 0 {
                    return Err(LookupError::NotFound { function: "XMATCH" });
                }
                Ok((compact[lo - 1].0 + 1) as i64)
            }
        }
    }
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
    fn xlookup_default_exact_hit() {
        let keys = vec![t("a"), t("b"), t("c")];
        let vals = vec![n(1.0), n(2.0), n(3.0)];
        let r = xlookup(
            &t("b"),
            &keys,
            &vals,
            None,
            XMatchMode::Exact,
            XSearchMode::FirstToLast,
        )
        .unwrap();
        assert_eq!(r, n(2.0));
    }

    #[test]
    fn xlookup_default_exact_miss_uses_fallback() {
        let keys = vec![t("a"), t("b")];
        let vals = vec![n(1.0), n(2.0)];
        let r = xlookup(
            &t("z"),
            &keys,
            &vals,
            Some(t("missing")),
            XMatchMode::Exact,
            XSearchMode::FirstToLast,
        )
        .unwrap();
        assert_eq!(r, t("missing"));
    }

    #[test]
    fn xlookup_default_exact_miss_no_fallback_errors() {
        let keys = vec![t("a")];
        let vals = vec![n(1.0)];
        let err = xlookup(
            &t("z"),
            &keys,
            &vals,
            None,
            XMatchMode::Exact,
            XSearchMode::FirstToLast,
        )
        .unwrap_err();
        assert!(matches!(err, LookupError::NotFound { .. }));
    }

    #[test]
    fn xlookup_shape_mismatch() {
        let keys = vec![t("a")];
        let vals = vec![n(1.0), n(2.0)];
        let err = xlookup(
            &t("a"),
            &keys,
            &vals,
            None,
            XMatchMode::Exact,
            XSearchMode::FirstToLast,
        )
        .unwrap_err();
        assert!(matches!(err, LookupError::ShapeMismatch { .. }));
    }

    #[test]
    fn xmatch_last_to_first_finds_rightmost() {
        let arr = vec![t("a"), t("b"), t("a"), t("c")];
        // First-to-last finds index 1, last-to-first finds 3.
        assert_eq!(
            xmatch(&t("a"), &arr, XMatchMode::Exact, XSearchMode::FirstToLast).unwrap(),
            1
        );
        assert_eq!(
            xmatch(&t("a"), &arr, XMatchMode::Exact, XSearchMode::LastToFirst).unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_next_smaller_linear() {
        let arr = vec![n(10.0), n(20.0), n(30.0)];
        // probe=25, nearest smaller=20 at position 2
        assert_eq!(
            xmatch(
                &n(25.0),
                &arr,
                XMatchMode::ExactOrNextSmaller,
                XSearchMode::FirstToLast,
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn xmatch_next_larger_linear() {
        let arr = vec![n(10.0), n(20.0), n(30.0)];
        // probe=25, nearest larger=30 at position 3
        assert_eq!(
            xmatch(
                &n(25.0),
                &arr,
                XMatchMode::ExactOrNextLarger,
                XSearchMode::FirstToLast,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_wildcard_matches_pattern() {
        let arr = vec![t("apple"), t("banana"), t("apricot")];
        assert_eq!(
            xmatch(
                &t("ap*"),
                &arr,
                XMatchMode::Wildcard,
                XSearchMode::FirstToLast,
            )
            .unwrap(),
            1
        );
        assert_eq!(
            xmatch(
                &t("a?ri*"),
                &arr,
                XMatchMode::Wildcard,
                XSearchMode::FirstToLast,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_binary_asc_exact_hit() {
        let arr = vec![n(1.0), n(3.0), n(5.0), n(7.0), n(9.0)];
        assert_eq!(
            xmatch(
                &n(5.0),
                &arr,
                XMatchMode::Exact,
                XSearchMode::BinarySortedAsc,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_binary_asc_next_smaller() {
        let arr = vec![n(1.0), n(3.0), n(5.0), n(7.0), n(9.0)];
        assert_eq!(
            xmatch(
                &n(6.0),
                &arr,
                XMatchMode::ExactOrNextSmaller,
                XSearchMode::BinarySortedAsc,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_binary_asc_next_larger() {
        let arr = vec![n(1.0), n(3.0), n(5.0), n(7.0), n(9.0)];
        assert_eq!(
            xmatch(
                &n(6.0),
                &arr,
                XMatchMode::ExactOrNextLarger,
                XSearchMode::BinarySortedAsc,
            )
            .unwrap(),
            4
        );
    }

    #[test]
    fn xmatch_binary_desc_exact_hit() {
        let arr = vec![n(9.0), n(7.0), n(5.0), n(3.0), n(1.0)];
        assert_eq!(
            xmatch(
                &n(5.0),
                &arr,
                XMatchMode::Exact,
                XSearchMode::BinarySortedDesc,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn xmatch_binary_desc_next_smaller() {
        let arr = vec![n(9.0), n(7.0), n(5.0), n(3.0), n(1.0)];
        // 4.0 → next smaller is 3.0 at descending-position 4.
        assert_eq!(
            xmatch(
                &n(4.0),
                &arr,
                XMatchMode::ExactOrNextSmaller,
                XSearchMode::BinarySortedDesc,
            )
            .unwrap(),
            4
        );
    }

    #[test]
    fn xmatch_excel_code_parsers() {
        assert_eq!(XMatchMode::from_excel_code(0).unwrap(), XMatchMode::Exact);
        assert_eq!(
            XMatchMode::from_excel_code(-1).unwrap(),
            XMatchMode::ExactOrNextSmaller
        );
        assert_eq!(
            XMatchMode::from_excel_code(1).unwrap(),
            XMatchMode::ExactOrNextLarger
        );
        assert_eq!(
            XMatchMode::from_excel_code(2).unwrap(),
            XMatchMode::Wildcard
        );
        assert!(XMatchMode::from_excel_code(9).is_err());

        assert_eq!(
            XSearchMode::from_excel_code(1).unwrap(),
            XSearchMode::FirstToLast
        );
        assert_eq!(
            XSearchMode::from_excel_code(-1).unwrap(),
            XSearchMode::LastToFirst
        );
        assert_eq!(
            XSearchMode::from_excel_code(2).unwrap(),
            XSearchMode::BinarySortedAsc
        );
        assert_eq!(
            XSearchMode::from_excel_code(-2).unwrap(),
            XSearchMode::BinarySortedDesc
        );
        assert!(XSearchMode::from_excel_code(99).is_err());
    }

    #[test]
    fn xlookup_na_probe_propagates() {
        let keys = vec![n(1.0)];
        let vals = vec![n(10.0)];
        let r = xlookup(
            &LookupValue::na(),
            &keys,
            &vals,
            None,
            XMatchMode::Exact,
            XSearchMode::FirstToLast,
        )
        .unwrap();
        assert!(r.is_na());
    }

    #[test]
    fn xmatch_empty_array_is_not_found() {
        let arr: Vec<LookupValue> = vec![];
        let err = xmatch(&t("a"), &arr, XMatchMode::Exact, XSearchMode::FirstToLast).unwrap_err();
        assert!(matches!(err, LookupError::NotFound { .. }));
    }
}
