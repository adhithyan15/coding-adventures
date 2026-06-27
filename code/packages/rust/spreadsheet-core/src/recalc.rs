//! Formula evaluation.

use crate::address::{CellAddress, CellRange, SheetId};
use crate::ast::{BinaryOp, FormulaAst, UnaryOp};
use crate::cell::CellValue;
use crate::dispatch::dispatch;
use crate::errors::SpreadsheetError;

/// Evaluate a formula AST against the workbook view.
///
/// `lookup` is an injected callback that returns the current value
/// of an arbitrary cell (the workbook owns the storage; we don't
/// need a mutable reference here). `current_sheet` is the sheet the
/// formula lives on (used to resolve sheet-local refs).
pub fn evaluate<F>(
    ast: &FormulaAst,
    current_sheet: SheetId,
    lookup: &F,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
{
    match ast {
        FormulaAst::Literal(v) => Ok(v.clone()),
        FormulaAst::Ref { sheet: None, addr } => Ok(lookup(current_sheet, *addr)),
        FormulaAst::Range { sheet: None, range } => evaluate_range(*range, current_sheet, lookup),
        // Cross-sheet references parse and round-trip, but resolving a sheet name
        // to a `SheetId` needs a workbook the evaluator doesn't hold yet, so they
        // evaluate to `#REF!` for now — a clean "not wired" signal, never a wrong
        // value. The next slice threads a name resolver through and reads the
        // target sheet.
        FormulaAst::Ref { sheet: Some(_), .. } | FormulaAst::Range { sheet: Some(_), .. } => {
            Err(SpreadsheetError::Ref)
        }
        FormulaAst::Percent(inner) => {
            let v = evaluate(inner, current_sheet, lookup)?.coerce_number()?;
            Ok(CellValue::Number(v / 100.0))
        }
        FormulaAst::Unary { op, operand } => {
            let v = evaluate(operand, current_sheet, lookup)?.coerce_number()?;
            match op {
                UnaryOp::Negate => Ok(CellValue::Number(-v)),
                UnaryOp::Plus => Ok(CellValue::Number(v)),
            }
        }
        FormulaAst::Binary { op, lhs, rhs } => {
            // Errors propagate left-first.
            let l = evaluate(lhs, current_sheet, lookup)?;
            let r = evaluate(rhs, current_sheet, lookup)?;
            apply_binary(*op, l, r)
        }
        FormulaAst::Call { name, args } => {
            // Lazy-evaluate the inlined logical forms (`IF`, `AND`,
            // `OR`) because Excel short-circuits them and skipping
            // matters for `IF(A1=0, 0, 1/A1)`-style guards.
            let upper = name.to_ascii_uppercase();
            if upper == "IF" {
                return eval_if_lazy(args, current_sheet, lookup);
            }
            if upper == "AND" || upper == "OR" {
                return eval_logical_short_circuit(&upper, args, current_sheet, lookup);
            }
            // IFERROR / IFNA: catch the inner error rather than
            // letting it propagate up before we see it.
            if upper == "IFERROR" || upper == "IFNA" {
                return eval_iferror_lazy(&upper, args, current_sheet, lookup);
            }
            // Normal eager call. Range arguments are expanded into
            // individual cell values so SUM(A1:A5) flattens to five
            // arguments rather than the multi-cell "#VALUE!"
            // surrogate from `evaluate_range`.
            let mut resolved = Vec::with_capacity(args.len());
            for a in args {
                if let FormulaAst::Range { sheet, range } = a {
                    // Cross-sheet range arg: same "#REF! until wired" rule as a
                    // standalone cross-sheet ref.
                    if sheet.is_some() {
                        return Err(SpreadsheetError::Ref);
                    }
                    // Reject an adversarially huge range before it can
                    // allocate billions of argument values.
                    if range.is_oversized() {
                        return Err(SpreadsheetError::Ref);
                    }
                    for addr in range.iter() {
                        resolved.push(lookup(current_sheet, addr));
                    }
                } else {
                    let v = evaluate(a, current_sheet, lookup)?;
                    resolved.push(v);
                }
            }
            dispatch(name, &resolved)
        }
    }
}

fn evaluate_range<F>(
    range: CellRange,
    sheet: SheetId,
    lookup: &F,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
{
    // For now we flatten a range into a single value if it's a
    // singleton; otherwise we... well, dispatch handles the
    // flattening from `Vec<CellValue>`. Since the AST passes a
    // `Range` as a single argument, dispatch needs to receive each
    // cell as a separate arg.
    //
    // Simplification: when a `Range` is evaluated as a standalone
    // expression (e.g. `=A1:A10` in a single cell), Excel implicitly
    // intersects to the same row/col; we surface `#VALUE!` to match
    // pre-365 Excel. When the range appears as a function arg, the
    // function-call evaluator flattens it via `flatten_range_args`.
    if range.is_oversized() {
        return Err(SpreadsheetError::Ref);
    }
    let cells: Vec<CellValue> = range.iter().map(|a| lookup(sheet, a)).collect();
    if cells.len() == 1 {
        return Ok(cells.into_iter().next().unwrap());
    }
    // Multi-cell range as a top-level value → #VALUE! pending Phase 2
    // dynamic-array spill handling.
    Err(SpreadsheetError::Value)
}

fn apply_binary(op: BinaryOp, l: CellValue, r: CellValue) -> Result<CellValue, SpreadsheetError> {
    // Error propagation: first error wins.
    if let CellValue::Error(e) = l {
        return Err(e);
    }
    if let CellValue::Error(e) = r {
        return Err(e);
    }
    if matches!(op, BinaryOp::Concat) {
        let lt = l.coerce_text()?;
        let rt = r.coerce_text()?;
        return Ok(CellValue::Text(lt + &rt));
    }
    if matches!(
        op,
        BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
    ) {
        return comparison(op, &l, &r);
    }
    let a = l.coerce_number()?;
    let b = r.coerce_number()?;
    let v = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => {
            if b == 0.0 {
                return Err(SpreadsheetError::DivZero);
            }
            a / b
        }
        BinaryOp::Pow => a.powf(b),
        BinaryOp::Concat | BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt
        | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => unreachable!(),
    };
    if v.is_nan() {
        return Err(SpreadsheetError::Num);
    }
    Ok(CellValue::Number(v))
}

fn comparison(op: BinaryOp, l: &CellValue, r: &CellValue) -> Result<CellValue, SpreadsheetError> {
    // Excel's type-ordered comparison: number < text < FALSE < TRUE.
    let order_l = type_rank(l);
    let order_r = type_rank(r);
    let result = if order_l != order_r {
        match op {
            BinaryOp::Eq => false,
            BinaryOp::Ne => true,
            BinaryOp::Lt => order_l < order_r,
            BinaryOp::Le => order_l <= order_r,
            BinaryOp::Gt => order_l > order_r,
            BinaryOp::Ge => order_l >= order_r,
            _ => unreachable!(),
        }
    } else {
        match (l, r) {
            (CellValue::Number(a), CellValue::Number(b)) => compare_op(op, a.partial_cmp(b)),
            (CellValue::Text(a), CellValue::Text(b)) => compare_op(
                op,
                Some(a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())),
            ),
            (CellValue::Boolean(a), CellValue::Boolean(b)) => compare_op(op, Some(a.cmp(b))),
            (CellValue::Empty, CellValue::Empty) => matches!(
                op,
                BinaryOp::Eq | BinaryOp::Le | BinaryOp::Ge
            ),
            _ => false,
        }
    };
    Ok(CellValue::Boolean(result))
}

fn type_rank(v: &CellValue) -> u8 {
    match v {
        CellValue::Empty => 0,
        CellValue::Number(_) => 1,
        CellValue::Text(_) => 2,
        CellValue::Boolean(_) => 3,
        CellValue::Error(_) => 4,
    }
}

fn compare_op(op: BinaryOp, ord: Option<core::cmp::Ordering>) -> bool {
    let Some(ord) = ord else { return false };
    use core::cmp::Ordering::*;
    match op {
        BinaryOp::Eq => ord == Equal,
        BinaryOp::Ne => ord != Equal,
        BinaryOp::Lt => ord == Less,
        BinaryOp::Le => ord != Greater,
        BinaryOp::Gt => ord == Greater,
        BinaryOp::Ge => ord != Less,
        _ => unreachable!(),
    }
}

fn eval_if_lazy<F>(
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
{
    if args.len() < 2 || args.len() > 3 {
        return Err(SpreadsheetError::Value);
    }
    let cond = evaluate(&args[0], sheet, lookup)?.coerce_bool()?;
    if cond {
        evaluate(&args[1], sheet, lookup)
    } else if args.len() == 3 {
        evaluate(&args[2], sheet, lookup)
    } else {
        Ok(CellValue::Boolean(false))
    }
}

/// `IFERROR(value, fallback)` and `IFNA(value, fallback)` need
/// special-case handling because the inner evaluation may return an
/// error; the engine's normal `Result<CellValue, _>` would propagate
/// the error past the IFERROR wrapper.
fn eval_iferror_lazy<F>(
    name: &str,
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
{
    if args.len() != 2 {
        return Err(SpreadsheetError::Value);
    }
    // Try the primary expression. If it errors, swap to the fallback
    // (for IFNA, only swap when the error is specifically #N/A).
    let primary = match evaluate(&args[0], sheet, lookup) {
        Ok(v) => v,
        Err(SpreadsheetError::NotAvailable) if name == "IFNA" => {
            return evaluate(&args[1], sheet, lookup);
        }
        Err(_) if name == "IFERROR" => return evaluate(&args[1], sheet, lookup),
        Err(e) if name == "IFNA" => return Err(e),
        Err(e) => return Err(e),
    };
    // A non-error value passes through, unless we want IFNA and the
    // value is the explicit #N/A literal.
    if name == "IFNA"
        && matches!(primary, CellValue::Error(SpreadsheetError::NotAvailable))
    {
        return evaluate(&args[1], sheet, lookup);
    }
    if name == "IFERROR" && matches!(primary, CellValue::Error(_)) {
        return evaluate(&args[1], sheet, lookup);
    }
    Ok(primary)
}

fn eval_logical_short_circuit<F>(
    op: &str,
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
{
    if args.is_empty() {
        return Err(SpreadsheetError::Value);
    }
    let want_true = op == "AND";
    for a in args {
        let b = evaluate(a, sheet, lookup)?.coerce_bool()?;
        if b != want_true {
            return Ok(CellValue::Boolean(!want_true));
        }
    }
    Ok(CellValue::Boolean(want_true))
}

/// Recursively walk an AST and emit the cell addresses it depends on.
/// Used by the workbook to build the dependency graph after parsing.
pub fn collect_refs(ast: &FormulaAst, current_sheet: SheetId, out: &mut Vec<(SheetId, CellAddress)>) {
    match ast {
        FormulaAst::Literal(_) => {}
        // Normalise away `$` markers: the dependency graph (like the cell store)
        // is keyed by position, so a `$A$1` precedent points at the same node as
        // `A1` — otherwise the edge would never match and `set_value` wouldn't
        // recompute a dependent that referenced it absolutely.
        FormulaAst::Ref { sheet: None, addr } => {
            out.push((current_sheet, addr.without_absolute()))
        }
        FormulaAst::Range { sheet: None, range } => {
            // Skip expansion of an oversized range: registering one
            // dependency per cell for `=SUM(A1:XFD1048576)` would
            // exhaust memory. Such a formula evaluates to `#REF!`
            // anyway (see the call-arg expansion and `evaluate_range`),
            // so it has no meaningful precedents to track.
            if !range.is_oversized() {
                for addr in range.iter() {
                    out.push((current_sheet, addr.without_absolute()));
                }
            }
        }
        // A cross-sheet reference evaluates to `#REF!` until the resolver lands,
        // so it has no precedent to register yet. The next slice resolves the
        // sheet name and pushes `(target_sheet, addr)` here.
        FormulaAst::Ref { sheet: Some(_), .. } | FormulaAst::Range { sheet: Some(_), .. } => {}
        FormulaAst::Unary { operand, .. } => collect_refs(operand, current_sheet, out),
        FormulaAst::Binary { lhs, rhs, .. } => {
            collect_refs(lhs, current_sheet, out);
            collect_refs(rhs, current_sheet, out);
        }
        FormulaAst::Percent(inner) => collect_refs(inner, current_sheet, out),
        FormulaAst::Call { args, .. } => {
            for a in args {
                collect_refs(a, current_sheet, out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn empty_lookup(_: SheetId, _: CellAddress) -> CellValue {
        CellValue::Empty
    }

    #[test]
    fn literal_evaluates_to_self() {
        let ast = parse("=42").unwrap();
        let r = evaluate(&ast, SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Number(42.0));
    }

    #[test]
    fn arithmetic_basic() {
        let r = evaluate(&parse("=1+2*3").unwrap(), SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Number(7.0));
    }

    #[test]
    fn division_by_zero() {
        let r = evaluate(&parse("=1/0").unwrap(), SheetId(0), &empty_lookup);
        assert_eq!(r, Err(SpreadsheetError::DivZero));
    }

    #[test]
    fn concat_operator() {
        let r = evaluate(&parse("=\"a\"&\"b\"").unwrap(), SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Text("ab".into()));
    }

    #[test]
    fn comparison_returns_boolean() {
        let r = evaluate(&parse("=3>2").unwrap(), SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Boolean(true));
    }

    #[test]
    fn function_call_to_sum() {
        let r = evaluate(
            &parse("=SUM(1, 2, 3)").unwrap(),
            SheetId(0),
            &empty_lookup,
        )
        .unwrap();
        assert_eq!(r, CellValue::Number(6.0));
    }

    #[test]
    fn lazy_if_does_not_divide_when_guarded() {
        // IF(0=0, 0, 1/0) — the else branch would error, but lazy IF
        // shouldn't evaluate it.
        let r = evaluate(
            &parse("=IF(0=0, 0, 1/0)").unwrap(),
            SheetId(0),
            &empty_lookup,
        )
        .unwrap();
        assert_eq!(r, CellValue::Number(0.0));
    }

    #[test]
    fn and_short_circuits_on_false() {
        // AND(FALSE, 1/0) — would error if eagerly evaluated.
        let r = evaluate(
            &parse("=AND(FALSE, 1/0)").unwrap(),
            SheetId(0),
            &empty_lookup,
        )
        .unwrap();
        assert_eq!(r, CellValue::Boolean(false));
    }

    #[test]
    fn cell_ref_via_injected_lookup() {
        let r = evaluate(&parse("=A1*2").unwrap(), SheetId(0), &|_, _| {
            CellValue::Number(5.0)
        })
        .unwrap();
        assert_eq!(r, CellValue::Number(10.0));
    }

    #[test]
    fn collect_refs_walks_tree() {
        let ast = parse("=SUM(A1, B2:B4) + C1").unwrap();
        let mut refs = Vec::new();
        collect_refs(&ast, SheetId(0), &mut refs);
        // A1, B2, B3, B4, C1 — five refs.
        assert_eq!(refs.len(), 5);
    }

    #[test]
    fn cross_sheet_ref_evaluates_to_ref_error_until_resolver_lands() {
        // A qualified reference parses and round-trips, but the evaluator can't
        // resolve a sheet name to a SheetId yet, so it yields #REF! — a clean
        // "not wired" signal, never a wrong value. The next slice replaces this.
        let r = evaluate(&parse("=Summary!A1").unwrap(), SheetId(0), &empty_lookup);
        assert_eq!(r, Err(SpreadsheetError::Ref));
        // …including a qualified range, standalone or as a SUM arg.
        assert_eq!(
            evaluate(&parse("=Summary!A1:B2").unwrap(), SheetId(0), &empty_lookup),
            Err(SpreadsheetError::Ref)
        );
        assert_eq!(
            evaluate(&parse("=SUM(Summary!A1:A4)").unwrap(), SheetId(0), &empty_lookup),
            Err(SpreadsheetError::Ref)
        );
    }

    #[test]
    fn collect_refs_skips_unresolved_cross_sheet_refs() {
        // A cross-sheet ref has no resolvable precedent yet, so it registers none
        // (its value is #REF! regardless). Same-sheet refs around it still count.
        let ast = parse("=A1 + Summary!B2 + C3").unwrap();
        let mut refs = Vec::new();
        collect_refs(&ast, SheetId(0), &mut refs);
        assert_eq!(refs.len(), 2); // A1 and C3 only
    }

    #[test]
    fn unary_negate() {
        let r = evaluate(&parse("=-5").unwrap(), SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Number(-5.0));
    }

    #[test]
    fn percent_postfix() {
        let r = evaluate(&parse("=50%").unwrap(), SheetId(0), &empty_lookup).unwrap();
        assert_eq!(r, CellValue::Number(0.5));
    }

    #[test]
    fn error_literal_passes_through_arithmetic() {
        let r = evaluate(&parse("=#N/A + 1").unwrap(), SheetId(0), &empty_lookup);
        assert_eq!(r, Err(SpreadsheetError::NotAvailable));
    }
}
