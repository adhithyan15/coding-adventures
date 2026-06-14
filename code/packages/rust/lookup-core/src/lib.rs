//! Lookup Core
//!
//! Phase 1 implementation for `code/specs/backend-crate-catalog.md` (the
//! `lookup-core` Layer-1 crate).  This crate provides Excel/Lotus/R-style
//! lookup and reference functions over a frontend-agnostic value type
//! [`LookupValue`].
//!
//! # Why a local value enum?
//!
//! The eventual `spreadsheet-core` crate will own the canonical `CellValue`
//! type, but that crate does not yet exist.  To keep `lookup-core` honest
//! about its data dependencies, this crate carries its own minimal value
//! enum.  When `spreadsheet-core` lands, a small `From<CellValue> for
//! LookupValue` adapter at the dispatch boundary will route values into this
//! crate without forcing it to depend on the full spreadsheet stack.
//!
//! # NA propagation
//!
//! NA in the *lookup_value* causes the function output to be NA (matches
//! Excel's `=VLOOKUP(NA(),…)` behaviour).  NA in the lookup *array* is
//! skipped — those cells never match a probe.  These rules come from
//! `code/specs/na-semantics.md`.
//!
//! # Numeric NA bit pattern
//!
//! Numeric NA inside `LookupValue::Number` is encoded via
//! [`r_vector::na_real`] / [`r_vector::is_na_real`], a quiet-NaN payload
//! distinct from plain `NaN`.  This lets us round-trip NA through
//! `LookupValue::Number` without inventing a separate variant.
//!
//! # Indexing convention
//!
//! All public APIs use **1-based** indexing (Excel's surface convention).
//! Internally we convert to 0-based at the function boundary so the rest of
//! the implementation is plain Rust.

pub mod choose;
pub mod index_match;
pub mod offset;
pub mod position;
pub mod vlookup;
pub mod xlookup;

pub use r_vector::{is_na_real, na_real};

/// A frontend-agnostic value used by every lookup function in this crate.
///
/// `Number` carries floating-point values; an NA is encoded by the
/// `r_vector::na_real()` bit pattern, so callers do not need a separate
/// variant.
#[derive(Debug, Clone, PartialEq)]
pub enum LookupValue {
    /// An empty cell.  Treated as "not a value" in comparisons — never
    /// matches a non-empty probe, matches another `Empty`.
    Empty,
    /// `TRUE`/`FALSE`.  Booleans only compare equal to other booleans.
    Boolean(bool),
    /// IEEE-754 double.  NA is encoded as the `na_real()` payload.
    Number(f64),
    /// UTF-8 text.  Approximate-match comparisons are ASCII case-insensitive.
    Text(String),
}

impl LookupValue {
    /// Convenience constructor for an NA number.
    pub fn na() -> Self {
        LookupValue::Number(na_real())
    }

    /// True iff this value is `Number(NA)` (the `na_real` bit pattern).
    pub fn is_na(&self) -> bool {
        match self {
            LookupValue::Number(value) => is_na_real(*value),
            _ => false,
        }
    }

    /// Short tag describing the variant, used in error messages.
    pub(crate) fn type_tag(&self) -> &'static str {
        match self {
            LookupValue::Empty => "empty",
            LookupValue::Boolean(_) => "boolean",
            LookupValue::Number(_) => "number",
            LookupValue::Text(_) => "text",
        }
    }
}

/// Errors emitted by lookup functions.  Each variant maps cleanly to an
/// Excel `#…!` error code at the dispatch boundary:
///
/// | `LookupError`         | Excel error | Notes                           |
/// |-----------------------|-------------|---------------------------------|
/// | `NotFound`            | `#N/A`      | Probe not present               |
/// | `OutOfRange`          | `#REF!`     | Index past end of array         |
/// | `BadParameter`        | `#VALUE!`   | E.g. negative height in OFFSET  |
/// | `ShapeMismatch`       | `#VALUE!`   | Arrays of incompatible length   |
/// | `TypeMismatch`        | `#VALUE!`   | Number vs text in approx match  |
#[derive(Debug, Clone, PartialEq)]
pub enum LookupError {
    /// The probe value is absent from the lookup array.
    NotFound { function: &'static str },
    /// A 1-based index is outside the valid range `1..=max`.
    OutOfRange {
        function: &'static str,
        index: i64,
        max: usize,
    },
    /// A parameter (e.g. `match_type`) had an invalid value.
    BadParameter {
        name: &'static str,
        value: String,
    },
    /// Two arrays disagree on length / shape.
    ShapeMismatch {
        expected: String,
        found: String,
    },
    /// Two values are of incompatible types for the requested comparison.
    TypeMismatch {
        function: &'static str,
        what: String,
    },
}

impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LookupError::NotFound { function } => write!(f, "{function}: no matching value (#N/A)"),
            LookupError::OutOfRange {
                function,
                index,
                max,
            } => write!(
                f,
                "{function}: index {index} is outside 1..={max} (#REF!)"
            ),
            LookupError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value} (#VALUE!)")
            }
            LookupError::ShapeMismatch { expected, found } => {
                write!(f, "shape mismatch: expected {expected}, found {found}")
            }
            LookupError::TypeMismatch { function, what } => {
                write!(f, "{function}: type mismatch ({what}) (#VALUE!)")
            }
        }
    }
}

impl std::error::Error for LookupError {}

/// Convenience alias for fallible lookup results.
pub type LookupResult<T> = Result<T, LookupError>;

// -----------------------------------------------------------------------
// Internal comparison helpers shared by all submodules.
//
// We centralise the comparison logic here so that VLOOKUP, MATCH, XLOOKUP
// and friends agree on what "equal" and "less than" mean — there is exactly
// one place in the crate where Excel parity lives.
// -----------------------------------------------------------------------

/// Exact equality comparison used by VLOOKUP-exact, MATCH-type-0, etc.
///
/// Numbers compare by IEEE equality (NA never compares equal to anything,
/// including itself — Excel's `=NA()=NA()` returns `#N/A`).  Text is
/// **case-insensitive ASCII** to match Excel's default.  Booleans only equal
/// other booleans; empties only equal other empties.
pub(crate) fn equal_excel(left: &LookupValue, right: &LookupValue) -> bool {
    match (left, right) {
        (LookupValue::Empty, LookupValue::Empty) => true,
        (LookupValue::Boolean(a), LookupValue::Boolean(b)) => a == b,
        (LookupValue::Number(a), LookupValue::Number(b)) => {
            if is_na_real(*a) || is_na_real(*b) {
                false
            } else {
                a == b
            }
        }
        (LookupValue::Text(a), LookupValue::Text(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

/// Ordering used for *approximate* lookups (VLOOKUP-TRUE, MATCH-1, MATCH--1,
/// XLOOKUP exact-or-next-*).  Returns:
///
/// - `Ok(Ordering)` when both values are comparable.
/// - `Err(TypeMismatch)` when the variants differ (Excel rule: approximate
///   match across types is `#N/A` in practice; we surface it as a typed
///   error and let the dispatcher map it back).
pub(crate) fn cmp_excel(
    function: &'static str,
    left: &LookupValue,
    right: &LookupValue,
) -> Result<std::cmp::Ordering, LookupError> {
    use std::cmp::Ordering;
    match (left, right) {
        (LookupValue::Empty, LookupValue::Empty) => Ok(Ordering::Equal),
        (LookupValue::Boolean(a), LookupValue::Boolean(b)) => Ok(a.cmp(b)),
        (LookupValue::Number(a), LookupValue::Number(b)) => {
            if is_na_real(*a) || is_na_real(*b) {
                Err(LookupError::TypeMismatch {
                    function,
                    what: "NA in approximate comparison".to_string(),
                })
            } else {
                // `total_cmp` keeps the function total even for ±0 and NaN.
                Ok(a.total_cmp(b))
            }
        }
        (LookupValue::Text(a), LookupValue::Text(b)) => {
            // ASCII case-insensitive ordering for parity with Excel sort.
            Ok(a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()))
        }
        (l, r) => Err(LookupError::TypeMismatch {
            function,
            what: format!("{} vs {}", l.type_tag(), r.type_tag()),
        }),
    }
}

/// Match a probe against a candidate using Excel-style wildcards:
///   `?` matches exactly one character (no Unicode awareness),
///   `*` matches zero or more characters,
///   `~?` and `~*` are literal `?`/`*`,
///   `~~` is a literal `~`.
///
/// Only applied when *both* sides are `Text`; for non-text the function
/// falls back to [`equal_excel`].
pub(crate) fn matches_wildcard(probe: &LookupValue, candidate: &LookupValue) -> bool {
    match (probe, candidate) {
        (LookupValue::Text(pattern), LookupValue::Text(target)) => {
            wildcard_match_ascii(pattern, target)
        }
        _ => equal_excel(probe, candidate),
    }
}

/// Recursive descent ASCII wildcard matcher.  We avoid regex / external
/// crates per the Layer-1 "no external deps" rule, and ASCII-fold both sides
/// to match Excel's case-insensitive behaviour.
fn wildcard_match_ascii(pattern: &str, target: &str) -> bool {
    // Tokenise the pattern into a stream of (LiteralChar | AnyChar | AnyRun)
    // tokens.  Building the token list once is `O(p)` and lets the matcher
    // collapse runs of `*` automatically.
    enum Tok {
        Lit(u8),
        AnyChar,
        AnyRun,
    }
    let bytes = pattern.as_bytes();
    let mut tokens: Vec<Tok> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'~' if i + 1 < bytes.len() => {
                tokens.push(Tok::Lit(bytes[i + 1].to_ascii_lowercase()));
                i += 2;
            }
            b'?' => {
                tokens.push(Tok::AnyChar);
                i += 1;
            }
            b'*' => {
                // Collapse consecutive `*` since `**` == `*`.
                if !matches!(tokens.last(), Some(Tok::AnyRun)) {
                    tokens.push(Tok::AnyRun);
                }
                i += 1;
            }
            _ => {
                tokens.push(Tok::Lit(b.to_ascii_lowercase()));
                i += 1;
            }
        }
    }

    let target_bytes: Vec<u8> = target.bytes().map(|b| b.to_ascii_lowercase()).collect();

    fn rec(toks: &[Tok], target: &[u8]) -> bool {
        match toks.first() {
            None => target.is_empty(),
            Some(Tok::Lit(c)) => !target.is_empty() && target[0] == *c && rec(&toks[1..], &target[1..]),
            Some(Tok::AnyChar) => !target.is_empty() && rec(&toks[1..], &target[1..]),
            Some(Tok::AnyRun) => {
                // Try matching zero characters first, then progressively
                // longer prefixes.  This is the standard backtracking
                // wildcard.  Worst case `O(p*t)`; fine for our use.
                if rec(&toks[1..], target) {
                    return true;
                }
                for k in 1..=target.len() {
                    if rec(&toks[1..], &target[k..]) {
                        return true;
                    }
                }
                false
            }
        }
    }

    rec(&tokens, &target_bytes)
}

/// Convert a 1-based index to 0-based, validating the bound.  Centralised so
/// that every "index outside 1..=n" error message reads the same.
pub(crate) fn one_based_to_zero(
    function: &'static str,
    index: i64,
    max: usize,
) -> Result<usize, LookupError> {
    if index < 1 || (index as usize) > max {
        return Err(LookupError::OutOfRange {
            function,
            index,
            max,
        });
    }
    Ok((index as usize) - 1)
}

// -----------------------------------------------------------------------
// Tests for helpers.
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_is_case_insensitive_for_text() {
        let a = LookupValue::Text("Apple".to_string());
        let b = LookupValue::Text("apple".to_string());
        assert!(equal_excel(&a, &b));
    }

    #[test]
    fn na_never_equals_itself() {
        let a = LookupValue::na();
        let b = LookupValue::na();
        assert!(!equal_excel(&a, &b));
    }

    #[test]
    fn wildcard_question_mark_matches_single_char() {
        let p = LookupValue::Text("a?c".to_string());
        let t = LookupValue::Text("abc".to_string());
        assert!(matches_wildcard(&p, &t));
        let t2 = LookupValue::Text("ac".to_string());
        assert!(!matches_wildcard(&p, &t2));
    }

    #[test]
    fn wildcard_star_matches_run() {
        let p = LookupValue::Text("a*c".to_string());
        assert!(matches_wildcard(
            &p,
            &LookupValue::Text("abc".to_string())
        ));
        assert!(matches_wildcard(
            &p,
            &LookupValue::Text("abbbbc".to_string())
        ));
        assert!(matches_wildcard(&p, &LookupValue::Text("ac".to_string())));
        assert!(!matches_wildcard(
            &p,
            &LookupValue::Text("abz".to_string())
        ));
    }

    #[test]
    fn wildcard_tilde_escapes_specials() {
        let p = LookupValue::Text("a~*c".to_string());
        assert!(matches_wildcard(
            &p,
            &LookupValue::Text("a*c".to_string())
        ));
        assert!(!matches_wildcard(
            &p,
            &LookupValue::Text("abc".to_string())
        ));
    }

    #[test]
    fn one_based_conversion_rejects_zero() {
        let err = one_based_to_zero("f", 0, 5).unwrap_err();
        match err {
            LookupError::OutOfRange { index, max, .. } => {
                assert_eq!(index, 0);
                assert_eq!(max, 5);
            }
            _ => panic!("wrong error"),
        }
    }

    #[test]
    fn one_based_conversion_accepts_in_range() {
        assert_eq!(one_based_to_zero("f", 1, 5).unwrap(), 0);
        assert_eq!(one_based_to_zero("f", 5, 5).unwrap(), 4);
    }

    #[test]
    fn display_strings_mention_excel_codes() {
        let e = LookupError::NotFound { function: "VLOOKUP" };
        assert!(format!("{e}").contains("#N/A"));
        let e = LookupError::OutOfRange {
            function: "INDEX",
            index: 9,
            max: 3,
        };
        assert!(format!("{e}").contains("#REF!"));
    }

    #[test]
    fn type_tag_is_stable() {
        assert_eq!(LookupValue::Empty.type_tag(), "empty");
        assert_eq!(LookupValue::Boolean(true).type_tag(), "boolean");
        assert_eq!(LookupValue::Number(1.0).type_tag(), "number");
        assert_eq!(LookupValue::Text("x".into()).type_tag(), "text");
    }
}
