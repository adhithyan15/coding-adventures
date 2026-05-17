//! Built-in spreadsheet functions.
//!
//! This module implements the six built-in functions defined in FE01:
//! `SUM`, `AVG`, `COUNT`, `MAX`, `MIN`, and `IF`.
//!
//! Each function receives a slice of evaluated [`CellValue`] arguments and
//! returns a single `CellValue`.  Range arguments (like `A1:C3`) are expanded
//! by the evaluator *before* calling these functions, so every element of the
//! slice is already a scalar value — no ranges appear here.
//!
//! # Design philosophy
//!
//! Spreadsheet functions are lenient by design.  `SUM(A1:A3)` where `A2`
//! contains text just skips that cell rather than erroring.  This matches the
//! behaviour of VisiCalc and Excel: numeric functions treat non-numeric values
//! as zero-or-skip depending on the function.
//!
//! Error propagation is handled *before* calling these functions: if any
//! argument is `CellValue::Error(e)`, the evaluator short-circuits and returns
//! that error without calling the builtin.

use crate::value::CellValue;
use crate::FormulaError;

/// Dispatch a built-in function call by name.
///
/// `name` is already upper-cased by the lexer.
/// `args` is the list of evaluated (scalar) arguments.
///
/// Returns `Err(FormulaError::Name)` for unknown function names.
pub fn call_builtin(name: &str, args: Vec<CellValue>) -> Result<CellValue, FormulaError> {
    match name {
        "SUM" => Ok(builtin_sum(&args)),
        "AVG" => Ok(builtin_avg(&args)),
        "COUNT" => Ok(builtin_count(&args)),
        "MAX" => Ok(builtin_max(&args)),
        "MIN" => Ok(builtin_min(&args)),
        "IF" => builtin_if(args),
        _ => Err(FormulaError::Name),
    }
}

// ── Individual function implementations ──────────────────────────────────────

/// `SUM(val_or_range...)` — sum of all numeric values.
///
/// Text cells and empty cells contribute 0 (they are silently skipped).
/// This matches Excel's behaviour: `SUM("hello", 3)` = 3.
fn builtin_sum(args: &[CellValue]) -> CellValue {
    let total: f64 = args.iter().filter_map(numeric_or_zero).sum();
    CellValue::Number(total)
}

/// `AVG(val_or_range...)` — arithmetic mean of numeric values.
///
/// Skips non-numeric cells when counting the denominator.
/// If *all* cells are non-numeric (or the list is empty), returns 0.
fn builtin_avg(args: &[CellValue]) -> CellValue {
    let nums: Vec<f64> = args.iter().filter_map(|v| v.as_number()).collect();
    if nums.is_empty() {
        CellValue::Number(0.0)
    } else {
        let sum: f64 = nums.iter().sum();
        CellValue::Number(sum / nums.len() as f64)
    }
}

/// `COUNT(val_or_range...)` — count of non-empty cells.
///
/// Both numbers and text cells are counted. Only `Empty` cells are excluded.
fn builtin_count(args: &[CellValue]) -> CellValue {
    let count = args.iter().filter(|v| !matches!(v, CellValue::Empty)).count();
    CellValue::Number(count as f64)
}

/// `MAX(val_or_range...)` — maximum numeric value.
///
/// Non-numeric cells are skipped. Returns 0 if there are no numeric values.
fn builtin_max(args: &[CellValue]) -> CellValue {
    let mut best: Option<f64> = None;
    for v in args {
        if let Some(n) = v.as_number() {
            best = Some(match best {
                None => n,
                Some(prev) => if n > prev { n } else { prev },
            });
        }
    }
    CellValue::Number(best.unwrap_or(0.0))
}

/// `MIN(val_or_range...)` — minimum numeric value.
///
/// Non-numeric cells are skipped. Returns 0 if there are no numeric values.
fn builtin_min(args: &[CellValue]) -> CellValue {
    let mut best: Option<f64> = None;
    for v in args {
        if let Some(n) = v.as_number() {
            best = Some(match best {
                None => n,
                Some(prev) => if n < prev { n } else { prev },
            });
        }
    }
    CellValue::Number(best.unwrap_or(0.0))
}

/// `IF(cond, true_val, false_val)` — conditional expression.
///
/// Requires exactly three arguments.  The condition is evaluated as a boolean:
/// 0 and `FALSE` are falsy; everything else (any non-zero number, `TRUE`, any
/// non-empty string) is truthy.
///
/// Returns `Err(FormulaError::Value)` if the wrong number of arguments is given.
fn builtin_if(mut args: Vec<CellValue>) -> Result<CellValue, FormulaError> {
    if args.len() != 3 {
        return Err(FormulaError::Value);
    }
    // Pull the three arguments out in order.
    let false_val = args.remove(2);
    let true_val = args.remove(1);
    let cond = args.remove(0);

    if cond.is_truthy() {
        Ok(true_val)
    } else {
        Ok(false_val)
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Return the numeric value of `v`, or `Some(0.0)` for `Empty` and `Text`.
///
/// This is the "lenient" coercion used by `SUM` — non-numeric cells count as 0
/// rather than causing an error.
fn numeric_or_zero(v: &CellValue) -> Option<f64> {
    match v {
        CellValue::Number(n) => Some(*n),
        CellValue::Bool(true) => Some(1.0),
        CellValue::Bool(false) => Some(0.0),
        CellValue::Empty => Some(0.0),
        CellValue::Text(_) => Some(0.0),
        CellValue::Error(_) => None, // errors are filtered out before we get here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64) -> CellValue { CellValue::Number(x) }
    fn t(s: &str) -> CellValue { CellValue::Text(s.to_string()) }
    fn empty() -> CellValue { CellValue::Empty }

    #[test]
    fn test_builtin_sum() {
        let r = call_builtin("SUM", vec![n(1.0), n(2.0), n(3.0)]).unwrap();
        assert_eq!(r, n(6.0));
    }

    #[test]
    fn test_builtin_sum_skips_text() {
        // Text is treated as 0 in SUM.
        let r = call_builtin("SUM", vec![n(1.0), t("hello"), n(3.0)]).unwrap();
        assert_eq!(r, n(4.0));
    }

    #[test]
    fn test_builtin_avg() {
        let r = call_builtin("AVG", vec![n(1.0), n(2.0), n(3.0)]).unwrap();
        assert_eq!(r, n(2.0));
    }

    #[test]
    fn test_builtin_avg_empty_returns_zero() {
        let r = call_builtin("AVG", vec![]).unwrap();
        assert_eq!(r, n(0.0));
    }

    #[test]
    fn test_builtin_count() {
        // Counts all non-empty cells.
        let r = call_builtin("COUNT", vec![n(1.0), t("hi"), empty()]).unwrap();
        assert_eq!(r, n(2.0));
    }

    #[test]
    fn test_builtin_max() {
        let r = call_builtin("MAX", vec![n(3.0), n(1.0), n(4.0), n(1.0), n(5.0)]).unwrap();
        assert_eq!(r, n(5.0));
    }

    #[test]
    fn test_builtin_min() {
        let r = call_builtin("MIN", vec![n(3.0), n(1.0), n(4.0), n(1.0), n(5.0)]).unwrap();
        assert_eq!(r, n(1.0));
    }

    #[test]
    fn test_builtin_if_true() {
        let r = call_builtin("IF", vec![n(1.0), n(2.0), n(3.0)]).unwrap();
        assert_eq!(r, n(2.0));
    }

    #[test]
    fn test_builtin_if_false() {
        let r = call_builtin("IF", vec![n(0.0), n(2.0), n(3.0)]).unwrap();
        assert_eq!(r, n(3.0));
    }

    #[test]
    fn test_builtin_if_wrong_arity() {
        assert!(call_builtin("IF", vec![n(1.0)]).is_err());
        assert!(call_builtin("IF", vec![n(1.0), n(2.0)]).is_err());
    }

    #[test]
    fn test_unknown_function() {
        let err = call_builtin("FOO", vec![]).unwrap_err();
        assert_eq!(err, FormulaError::Name);
    }
}
