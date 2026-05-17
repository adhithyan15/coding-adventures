//! Formula evaluator — walks the AST and produces a CellValue.
//!
//! Given an [`Expr`] produced by the parser and a lookup function that maps
//! cell addresses to their current values, the evaluator recursively computes
//! the result.
//!
//! # Error propagation
//!
//! Errors short-circuit: if any sub-expression produces an error, that error
//! bubbles up immediately without evaluating the rest of the expression.  For
//! example, if `A1` contains `#DIV/0!`, then `=A1 + 1` also becomes `#DIV/0!`
//! without attempting the addition.
//!
//! # Range handling
//!
//! Ranges (`A1:C3`) are *not* valid as standalone expressions — they can only
//! appear as arguments to functions like `SUM`.  The evaluator expands a range
//! argument into a list of cell values before passing them to the builtin.
//!
//! # Arithmetic semantics
//!
//! - Addition/subtraction/multiplication require numeric operands.  Text
//!   operands produce `#VALUE!`.
//! - Division by zero produces `#DIV/0!`.
//! - Unary negation of a non-number produces `#VALUE!`.

use crate::addr::CellAddr;
use crate::builtins::call_builtin;
use crate::parser::Expr;
use crate::value::CellValue;
use crate::FormulaError;

/// Evaluate an expression.
///
/// `lookup` is a closure that maps a `CellAddr` to the current value of that
/// cell (or `CellValue::Empty` if the cell has never been set).
///
/// This signature allows the evaluator to be tested independently of the full
/// `FormulaEngine` by passing a custom lookup function.
pub fn eval<F>(expr: &Expr, lookup: &F) -> CellValue
where
    F: Fn(&CellAddr) -> CellValue,
{
    match expr {
        // ── Literals ─────────────────────────────────────────────────────
        Expr::Number(n) => CellValue::Number(*n),
        Expr::Str(s) => CellValue::Text(s.clone()),
        Expr::Bool(b) => CellValue::Bool(*b),

        // ── Cell reference ────────────────────────────────────────────────
        Expr::CellRef(addr_str) => {
            match CellAddr::parse(addr_str) {
                Ok(addr) => lookup(&addr),
                Err(e) => CellValue::Error(e),
            }
        }

        // ── Range as standalone — this is a usage error ───────────────────
        // Ranges are only valid inside function arguments; we expand them
        // there.  If someone writes =A1:B2 without a wrapping function, that
        // is a #VALUE! error.
        Expr::Range(_, _) => CellValue::Error(FormulaError::Value),

        // ── Unary negation ────────────────────────────────────────────────
        Expr::Neg(inner) => {
            let v = eval(inner, lookup);
            match v {
                CellValue::Number(n) => CellValue::Number(-n),
                CellValue::Error(e) => CellValue::Error(e),
                _ => CellValue::Error(FormulaError::Value),
            }
        }

        // ── Binary addition ───────────────────────────────────────────────
        Expr::Add(left, right) => {
            let lv = eval(left, lookup);
            let rv = eval(right, lookup);
            binary_numeric(lv, rv, |a, b| a + b)
        }

        // ── Binary subtraction ────────────────────────────────────────────
        Expr::Sub(left, right) => {
            let lv = eval(left, lookup);
            let rv = eval(right, lookup);
            binary_numeric(lv, rv, |a, b| a - b)
        }

        // ── Binary multiplication ─────────────────────────────────────────
        Expr::Mul(left, right) => {
            let lv = eval(left, lookup);
            let rv = eval(right, lookup);
            binary_numeric(lv, rv, |a, b| a * b)
        }

        // ── Binary division ───────────────────────────────────────────────
        Expr::Div(left, right) => {
            let lv = eval(left, lookup);
            let rv = eval(right, lookup);
            // Check for division by zero before the general case.
            match (&lv, &rv) {
                (CellValue::Error(e), _) => CellValue::Error(e.clone()),
                (_, CellValue::Error(e)) => CellValue::Error(e.clone()),
                (_, CellValue::Number(d)) if *d == 0.0 => {
                    CellValue::Error(FormulaError::DivZero)
                }
                _ => binary_numeric(lv, rv, |a, b| a / b),
            }
        }

        // ── Function call ─────────────────────────────────────────────────
        Expr::FuncCall { name, args } => {
            // Evaluate each argument, expanding ranges into individual cells.
            let mut evaled: Vec<CellValue> = Vec::new();
            for arg in args {
                match arg {
                    Expr::Range(start_str, end_str) => {
                        // Expand the range to individual cell values.
                        match expand_range(start_str, end_str, lookup) {
                            Ok(cells) => evaled.extend(cells),
                            Err(e) => return CellValue::Error(e),
                        }
                    }
                    other => {
                        evaled.push(eval(other, lookup));
                    }
                }
            }

            // Short-circuit if any argument is an error.
            // Exception: IF needs to see all three args to decide, but we
            // still propagate errors through IF — if a condition or chosen
            // branch is an error, the result is an error.
            for v in &evaled {
                if let CellValue::Error(e) = v {
                    return CellValue::Error(e.clone());
                }
            }

            // Dispatch to the builtin.
            match call_builtin(name, evaled) {
                Ok(v) => v,
                Err(e) => CellValue::Error(e),
            }
        }
    }
}

/// Apply a numeric binary operation.
///
/// If either operand is an error, propagate the error.
/// If either operand is not a number, return `#VALUE!`.
fn binary_numeric<F>(left: CellValue, right: CellValue, op: F) -> CellValue
where
    F: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (CellValue::Error(e), _) => CellValue::Error(e),
        (_, CellValue::Error(e)) => CellValue::Error(e),
        (CellValue::Number(a), CellValue::Number(b)) => CellValue::Number(op(a, b)),
        _ => CellValue::Error(FormulaError::Value),
    }
}

/// Expand a range like `A1:C3` into the `CellValue` of every cell in the
/// rectangle, in column-major then row-major order.
///
/// Column-major order means: for each column (A, B, C), iterate all rows
/// (1, 2, 3).  So `A1:B2` yields A1, A2, B1, B2.
fn expand_range<F>(
    start_str: &str,
    end_str: &str,
    lookup: &F,
) -> Result<Vec<CellValue>, FormulaError>
where
    F: Fn(&CellAddr) -> CellValue,
{
    let start = CellAddr::parse(start_str)?;
    let end = CellAddr::parse(end_str)?;

    // Allow the user to write the range in any order; normalise so that
    // start ≤ end in both dimensions.
    let col_lo = start.col.min(end.col);
    let col_hi = start.col.max(end.col);
    let row_lo = start.row.min(end.row);
    let row_hi = start.row.max(end.row);

    let mut cells = Vec::new();
    for col in col_lo..=col_hi {
        for row in row_lo..=row_hi {
            let addr = CellAddr { col, row };
            cells.push(lookup(&addr));
        }
    }
    Ok(cells)
}

/// Collect all cell addresses referenced by an expression.
///
/// Used by the engine to build the dependency graph.  A range `A1:C3` adds
/// every cell in the rectangle to the dependency set.
pub fn collect_refs(expr: &Expr) -> Vec<CellAddr> {
    let mut refs = Vec::new();
    collect_refs_inner(expr, &mut refs);
    refs
}

fn collect_refs_inner(expr: &Expr, out: &mut Vec<CellAddr>) {
    match expr {
        Expr::CellRef(s) => {
            if let Ok(addr) = CellAddr::parse(s) {
                out.push(addr);
            }
        }
        Expr::Range(start_str, end_str) => {
            if let (Ok(start), Ok(end)) = (CellAddr::parse(start_str), CellAddr::parse(end_str)) {
                let col_lo = start.col.min(end.col);
                let col_hi = start.col.max(end.col);
                let row_lo = start.row.min(end.row);
                let row_hi = start.row.max(end.row);
                for col in col_lo..=col_hi {
                    for row in row_lo..=row_hi {
                        out.push(CellAddr { col, row });
                    }
                }
            }
        }
        Expr::Add(l, r) | Expr::Sub(l, r) | Expr::Mul(l, r) | Expr::Div(l, r) => {
            collect_refs_inner(l, out);
            collect_refs_inner(r, out);
        }
        Expr::Neg(inner) => collect_refs_inner(inner, out),
        Expr::FuncCall { args, .. } => {
            for arg in args {
                collect_refs_inner(arg, out);
            }
        }
        Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn eval_formula(formula: &str, lookup: &impl Fn(&CellAddr) -> CellValue) -> CellValue {
        let tokens = tokenize(formula).expect("tokenize failed");
        let expr = parse(tokens).expect("parse failed");
        eval(&expr, lookup)
    }

    fn no_cells(_: &CellAddr) -> CellValue { CellValue::Empty }

    #[test]
    fn test_eval_number() {
        let v = eval_formula("42", &no_cells);
        assert_eq!(v, CellValue::Number(42.0));
    }

    #[test]
    fn test_eval_addition() {
        let v = eval_formula("1 + 2", &no_cells);
        assert_eq!(v, CellValue::Number(3.0));
    }

    #[test]
    fn test_eval_division_by_zero() {
        let v = eval_formula("1 / 0", &no_cells);
        assert_eq!(v, CellValue::Error(FormulaError::DivZero));
    }

    #[test]
    fn test_eval_cell_ref() {
        let lookup = |addr: &CellAddr| {
            if addr.col == 0 && addr.row == 1 {
                CellValue::Number(5.0)
            } else {
                CellValue::Empty
            }
        };
        let v = eval_formula("A1 * 2", &lookup);
        assert_eq!(v, CellValue::Number(10.0));
    }

    #[test]
    fn test_eval_error_propagation() {
        let lookup = |_: &CellAddr| CellValue::Error(FormulaError::DivZero);
        let v = eval_formula("A1 + 1", &lookup);
        assert_eq!(v, CellValue::Error(FormulaError::DivZero));
    }

    #[test]
    fn test_eval_range_standalone_is_error() {
        let v = eval_formula("A1:B2", &no_cells);
        assert_eq!(v, CellValue::Error(FormulaError::Value));
    }

    #[test]
    fn test_collect_refs_simple() {
        let tokens = tokenize("A1 + B2").unwrap();
        let expr = parse(tokens).unwrap();
        let refs = collect_refs(&expr);
        let strs: Vec<String> = refs.iter().map(|a| a.to_addr_string()).collect();
        assert!(strs.contains(&"A1".to_string()));
        assert!(strs.contains(&"B2".to_string()));
    }

    #[test]
    fn test_collect_refs_range() {
        let tokens = tokenize("SUM(A1:A3)").unwrap();
        let expr = parse(tokens).unwrap();
        let refs = collect_refs(&expr);
        assert_eq!(refs.len(), 3); // A1, A2, A3
    }
}
