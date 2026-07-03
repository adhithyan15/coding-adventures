//! Formula evaluation.

use crate::address::{CellAddress, CellRange, SheetId};
use crate::ast::{BinaryOp, FormulaAst, UnaryOp};
use crate::cell::CellValue;
use crate::dispatch::dispatch;
use crate::errors::SpreadsheetError;

/// Resolve a reference's sheet to a concrete [`SheetId`]: an unqualified ref
/// (`None`) resolves to `current_sheet`; a qualified ref (`Some(name)`) is looked
/// up via `resolve`, which returns `None` for an unknown sheet name — the caller
/// then yields `#REF!`.
fn resolve_sheet<G>(sheet: &Option<String>, current_sheet: SheetId, resolve: &G) -> Option<SheetId>
where
    G: Fn(&str) -> Option<SheetId>,
{
    match sheet {
        None => Some(current_sheet),
        Some(name) => resolve(name),
    }
}

/// Evaluate a formula AST against the workbook view.
///
/// `lookup` is an injected callback that returns the current value of an
/// arbitrary cell (the workbook owns the storage; we don't need a mutable
/// reference here). `current_sheet` is the sheet the formula lives on (resolves
/// unqualified refs). `resolve` maps a sheet *name* to its [`SheetId`] so a
/// cross-sheet reference (`Summary!A1`) reads the target sheet; it returns `None`
/// for an unknown sheet, which becomes `#REF!`.
pub fn evaluate<F, G>(
    ast: &FormulaAst,
    current_sheet: SheetId,
    lookup: &F,
    resolve: &G,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
    G: Fn(&str) -> Option<SheetId>,
{
    match ast {
        FormulaAst::Literal(v) => Ok(v.clone()),
        FormulaAst::Ref { sheet, addr } => match resolve_sheet(sheet, current_sheet, resolve) {
            Some(sid) => Ok(lookup(sid, *addr)),
            None => Err(SpreadsheetError::Ref), // unknown sheet name
        },
        FormulaAst::Range { sheet, range } => match resolve_sheet(sheet, current_sheet, resolve) {
            Some(sid) => evaluate_range(*range, sid, lookup),
            None => Err(SpreadsheetError::Ref),
        },
        FormulaAst::Percent(inner) => {
            let v = evaluate(inner, current_sheet, lookup, resolve)?.coerce_number()?;
            Ok(CellValue::Number(v / 100.0))
        }
        FormulaAst::Unary { op, operand } => {
            let v = evaluate(operand, current_sheet, lookup, resolve)?.coerce_number()?;
            match op {
                UnaryOp::Negate => Ok(CellValue::Number(-v)),
                UnaryOp::Plus => Ok(CellValue::Number(v)),
            }
        }
        FormulaAst::Binary { op, lhs, rhs } => {
            // Errors propagate left-first.
            let l = evaluate(lhs, current_sheet, lookup, resolve)?;
            let r = evaluate(rhs, current_sheet, lookup, resolve)?;
            apply_binary(*op, l, r)
        }
        FormulaAst::Call { name, args } => {
            // Lazy-evaluate the inlined logical forms (`IF`, `AND`,
            // `OR`) because Excel short-circuits them and skipping
            // matters for `IF(A1=0, 0, 1/A1)`-style guards.
            let upper = name.to_ascii_uppercase();
            if upper == "IF" {
                return eval_if_lazy(args, current_sheet, lookup, resolve);
            }
            if upper == "AND" || upper == "OR" {
                return eval_logical_short_circuit(&upper, args, current_sheet, lookup, resolve);
            }
            // IFERROR / IFNA: catch the inner error rather than
            // letting it propagate up before we see it.
            if upper == "IFERROR" || upper == "IFNA" {
                return eval_iferror_lazy(&upper, args, current_sheet, lookup, resolve);
            }
            // Normal eager call. Range arguments are expanded into
            // individual cell values so SUM(A1:A5) flattens to five
            // arguments rather than the multi-cell "#VALUE!"
            // surrogate from `evaluate_range`.
            let mut resolved = Vec::with_capacity(args.len());
            for a in args {
                if let FormulaAst::Range { sheet, range } = a {
                    // Resolve the range's sheet (the cell's own, or a named one).
                    let Some(sid) = resolve_sheet(sheet, current_sheet, resolve) else {
                        return Err(SpreadsheetError::Ref); // unknown sheet name
                    };
                    // Reject an adversarially huge range before it can
                    // allocate billions of argument values.
                    if range.is_oversized() {
                        return Err(SpreadsheetError::Ref);
                    }
                    for addr in range.iter() {
                        resolved.push(lookup(sid, addr));
                    }
                } else {
                    let v = evaluate(a, current_sheet, lookup, resolve)?;
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

fn eval_if_lazy<F, G>(
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
    resolve: &G,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
    G: Fn(&str) -> Option<SheetId>,
{
    if args.len() < 2 || args.len() > 3 {
        return Err(SpreadsheetError::Value);
    }
    let cond = evaluate(&args[0], sheet, lookup, resolve)?.coerce_bool()?;
    if cond {
        evaluate(&args[1], sheet, lookup, resolve)
    } else if args.len() == 3 {
        evaluate(&args[2], sheet, lookup, resolve)
    } else {
        Ok(CellValue::Boolean(false))
    }
}

/// `IFERROR(value, fallback)` and `IFNA(value, fallback)` need
/// special-case handling because the inner evaluation may return an
/// error; the engine's normal `Result<CellValue, _>` would propagate
/// the error past the IFERROR wrapper.
fn eval_iferror_lazy<F, G>(
    name: &str,
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
    resolve: &G,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
    G: Fn(&str) -> Option<SheetId>,
{
    if args.len() != 2 {
        return Err(SpreadsheetError::Value);
    }
    // Try the primary expression. If it errors, swap to the fallback
    // (for IFNA, only swap when the error is specifically #N/A).
    let primary = match evaluate(&args[0], sheet, lookup, resolve) {
        Ok(v) => v,
        Err(SpreadsheetError::NotAvailable) if name == "IFNA" => {
            return evaluate(&args[1], sheet, lookup, resolve);
        }
        Err(_) if name == "IFERROR" => return evaluate(&args[1], sheet, lookup, resolve),
        Err(e) if name == "IFNA" => return Err(e),
        Err(e) => return Err(e),
    };
    // A non-error value passes through, unless we want IFNA and the
    // value is the explicit #N/A literal.
    if name == "IFNA"
        && matches!(primary, CellValue::Error(SpreadsheetError::NotAvailable))
    {
        return evaluate(&args[1], sheet, lookup, resolve);
    }
    if name == "IFERROR" && matches!(primary, CellValue::Error(_)) {
        return evaluate(&args[1], sheet, lookup, resolve);
    }
    Ok(primary)
}

fn eval_logical_short_circuit<F, G>(
    op: &str,
    args: &[FormulaAst],
    sheet: SheetId,
    lookup: &F,
    resolve: &G,
) -> Result<CellValue, SpreadsheetError>
where
    F: Fn(SheetId, CellAddress) -> CellValue,
    G: Fn(&str) -> Option<SheetId>,
{
    if args.is_empty() {
        return Err(SpreadsheetError::Value);
    }
    let want_true = op == "AND";
    for a in args {
        let b = evaluate(a, sheet, lookup, resolve)?.coerce_bool()?;
        if b != want_true {
            return Ok(CellValue::Boolean(!want_true));
        }
    }
    Ok(CellValue::Boolean(want_true))
}

/// Recursively walk an AST and emit the `(sheet, cell)` nodes it depends on.
/// Used by the workbook to build the dependency graph after parsing. `resolve`
/// maps a sheet name to its [`SheetId`] so a cross-sheet reference registers an
/// edge into the *target* sheet (the dependency graph is cross-sheet); a
/// reference to an unknown sheet evaluates to `#REF!` and registers nothing.
pub fn collect_refs<G>(
    ast: &FormulaAst,
    current_sheet: SheetId,
    resolve: &G,
    out: &mut Vec<(SheetId, CellAddress)>,
) where
    G: Fn(&str) -> Option<SheetId>,
{
    match ast {
        FormulaAst::Literal(_) => {}
        // Normalise away `$` markers: the dependency graph (like the cell store)
        // is keyed by position, so a `$A$1` precedent points at the same node as
        // `A1` — otherwise the edge would never match and `set_value` wouldn't
        // recompute a dependent that referenced it absolutely. A qualified ref
        // registers against its *target* sheet; an unknown sheet name registers
        // nothing (the formula is `#REF!`).
        FormulaAst::Ref { sheet, addr } => {
            if let Some(sid) = resolve_sheet(sheet, current_sheet, resolve) {
                out.push((sid, addr.without_absolute()));
            }
        }
        FormulaAst::Range { sheet, range } => {
            // Skip expansion of an oversized range: registering one
            // dependency per cell for `=SUM(A1:XFD1048576)` would
            // exhaust memory. Such a formula evaluates to `#REF!`
            // anyway (see the call-arg expansion and `evaluate_range`),
            // so it has no meaningful precedents to track.
            if let Some(sid) = resolve_sheet(sheet, current_sheet, resolve) {
                if !range.is_oversized() {
                    for addr in range.iter() {
                        out.push((sid, addr.without_absolute()));
                    }
                }
            }
        }
        FormulaAst::Unary { operand, .. } => collect_refs(operand, current_sheet, resolve, out),
        FormulaAst::Binary { lhs, rhs, .. } => {
            collect_refs(lhs, current_sheet, resolve, out);
            collect_refs(rhs, current_sheet, resolve, out);
        }
        FormulaAst::Percent(inner) => collect_refs(inner, current_sheet, resolve, out),
        FormulaAst::Call { args, .. } => {
            for a in args {
                collect_refs(a, current_sheet, resolve, out);
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

    /// A resolver for a single-sheet world: every sheet name is unknown, so a
    /// cross-sheet reference becomes `#REF!`. (Cross-sheet *success* is exercised
    /// at the workbook level, where real sheets exist.)
    fn no_sheets(_: &str) -> Option<SheetId> {
        None
    }

    #[test]
    fn literal_evaluates_to_self() {
        let ast = parse("=42").unwrap();
        let r = evaluate(&ast, SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Number(42.0));
    }

    #[test]
    fn arithmetic_basic() {
        let r = evaluate(&parse("=1+2*3").unwrap(), SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Number(7.0));
    }

    #[test]
    fn division_by_zero() {
        let r = evaluate(&parse("=1/0").unwrap(), SheetId(0), &empty_lookup, &no_sheets);
        assert_eq!(r, Err(SpreadsheetError::DivZero));
    }

    #[test]
    fn concat_operator() {
        let r = evaluate(&parse("=\"a\"&\"b\"").unwrap(), SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Text("ab".into()));
    }

    #[test]
    fn comparison_returns_boolean() {
        let r = evaluate(&parse("=3>2").unwrap(), SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Boolean(true));
    }

    #[test]
    fn function_call_to_sum() {
        let r = evaluate(
            &parse("=SUM(1, 2, 3)").unwrap(),
            SheetId(0),
            &empty_lookup,
            &no_sheets,
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
            &no_sheets,
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
            &no_sheets,
        )
        .unwrap();
        assert_eq!(r, CellValue::Boolean(false));
    }

    #[test]
    fn cell_ref_via_injected_lookup() {
        let r = evaluate(&parse("=A1*2").unwrap(), SheetId(0), &|_, _| {
            CellValue::Number(5.0)
        }, &no_sheets)
        .unwrap();
        assert_eq!(r, CellValue::Number(10.0));
    }

    #[test]
    fn collect_refs_walks_tree() {
        let ast = parse("=SUM(A1, B2:B4) + C1").unwrap();
        let mut refs = Vec::new();
        collect_refs(&ast, SheetId(0), &no_sheets, &mut refs);
        // A1, B2, B3, B4, C1 — five refs.
        assert_eq!(refs.len(), 5);
    }

    #[test]
    fn cross_sheet_ref_to_unknown_sheet_is_ref_error() {
        // A reference to a sheet that doesn't exist resolves to #REF! — never a
        // wrong value, and never a panic. (no_sheets resolves every name to None.)
        let r = evaluate(&parse("=Summary!A1").unwrap(), SheetId(0), &empty_lookup, &no_sheets);
        assert_eq!(r, Err(SpreadsheetError::Ref));
        // …including a qualified range, standalone or as a SUM arg.
        assert_eq!(
            evaluate(&parse("=Summary!A1:B2").unwrap(), SheetId(0), &empty_lookup, &no_sheets),
            Err(SpreadsheetError::Ref)
        );
        assert_eq!(
            evaluate(&parse("=SUM(Summary!A1:A4)").unwrap(), SheetId(0), &empty_lookup, &no_sheets),
            Err(SpreadsheetError::Ref)
        );
    }

    #[test]
    fn cross_sheet_ref_reads_the_resolved_target_sheet() {
        // A resolver that knows "Summary" = sheet 1, and a lookup that returns 7
        // for any cell on sheet 1 (and 0 elsewhere). =Summary!A1 must read sheet 1.
        let resolve = |name: &str| (name == "Summary").then_some(SheetId(1));
        let lookup = |sid: SheetId, _a: CellAddress| {
            CellValue::Number(if sid == SheetId(1) { 7.0 } else { 0.0 })
        };
        assert_eq!(
            evaluate(&parse("=Summary!A1").unwrap(), SheetId(0), &lookup, &resolve),
            Ok(CellValue::Number(7.0))
        );
        // A qualified range in SUM flattens against the target sheet: 3 cells × 7.
        assert_eq!(
            evaluate(&parse("=SUM(Summary!A1:A3)").unwrap(), SheetId(0), &lookup, &resolve),
            Ok(CellValue::Number(21.0))
        );
        // An unqualified ref in the same formula still reads the current sheet (0).
        assert_eq!(
            evaluate(&parse("=A1+Summary!A1").unwrap(), SheetId(0), &lookup, &resolve),
            Ok(CellValue::Number(7.0)) // 0 (sheet 0) + 7 (sheet 1)
        );
    }

    #[test]
    fn collect_refs_registers_cross_sheet_precedents_against_the_target() {
        // With "Summary" → sheet 1, a cross-sheet ref registers an edge into sheet
        // 1, while the same-sheet refs register against the current sheet (0).
        let resolve = |name: &str| (name == "Summary").then_some(SheetId(1));
        let ast = parse("=A1 + Summary!B2 + C3").unwrap();
        let mut refs = Vec::new();
        collect_refs(&ast, SheetId(0), &resolve, &mut refs);
        assert!(refs.contains(&(SheetId(0), CellAddress::new(1, 1)))); // A1 on sheet 0
        assert!(refs.contains(&(SheetId(1), CellAddress::new(2, 2)))); // Summary!B2 on sheet 1
        assert!(refs.contains(&(SheetId(0), CellAddress::new(3, 3)))); // C3 on sheet 0
        assert_eq!(refs.len(), 3);
        // An unknown sheet registers no precedent (the ref is #REF!).
        let mut refs2 = Vec::new();
        collect_refs(&parse("=Nope!A1").unwrap(), SheetId(0), &no_sheets, &mut refs2);
        assert!(refs2.is_empty());
    }

    #[test]
    fn unary_negate() {
        let r = evaluate(&parse("=-5").unwrap(), SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Number(-5.0));
    }

    #[test]
    fn percent_postfix() {
        let r = evaluate(&parse("=50%").unwrap(), SheetId(0), &empty_lookup, &no_sheets).unwrap();
        assert_eq!(r, CellValue::Number(0.5));
    }

    #[test]
    fn error_literal_passes_through_arithmetic() {
        let r = evaluate(&parse("=#N/A + 1").unwrap(), SheetId(0), &empty_lookup, &no_sheets);
        assert_eq!(r, Err(SpreadsheetError::NotAvailable));
    }
}
