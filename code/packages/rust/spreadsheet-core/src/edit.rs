//! Structural edits — insert/delete rows & columns — and the reference
//! arithmetic that keeps formulas pointing at the right cells.
//!
//! This module is the *pure* substrate of the insert/delete feature: given a
//! [`StructuralEdit`], it tells you where any address, range, or whole formula
//! moves to (or that it was destroyed). It mutates nothing and touches no
//! workbook state, so it is exhaustively unit-testable in isolation; a later
//! layer wires these transforms into [`Workbook`](crate::Workbook) (relocate the
//! cells, rewrite each formula's AST, rebuild the dependency graph, recalc).
//!
//! # The model
//!
//! A spreadsheet edit like "insert 2 rows above row 5" relabels every cell at or
//! below row 5 (they slide down by 2) and, crucially, *rewrites every formula*
//! so a reference that pointed at the old `A5` now points at the new `A7` — the
//! value the user sees must not change just because rows moved beneath it.
//!
//! Two rules are easy to get wrong, so they're stated up front:
//!
//! 1. **Structural edits shift absolute references too.** `$A$1` is "absolute"
//!    only against *copy/paste* ([`CellAddress::shift`]); inserting a row above
//!    it still makes it `$A$2`. The grid physically moved, so the reference
//!    must follow. Absolute flags are *preserved*, never *exempted*.
//!
//! 2. **A reference to a deleted line becomes `#REF!`.** If you delete the row a
//!    formula points at, there is no longer a cell to point at — the reference
//!    is replaced by the `#REF!` error literal, which then propagates through
//!    the formula exactly like any other error.
//!
//! ```text
//!   insert 1 row at 3:        delete row 3:
//!     row 1  A1                 row 1  A1
//!     row 2  A2                 row 2  A2
//!     row 3  ░░ (new blank)     row 3  A4  (A3 destroyed → #REF!)
//!     row 4  A3  (was row 3)    row 4  A5
//!     row 5  A4  (was row 4)
//! ```

use crate::address::{CellAddress, CellRange};
use crate::ast::FormulaAst;
use crate::cell::CellValue;
use crate::errors::SpreadsheetError;

/// A structural change to the grid's row/column layout. `at` is the 1-based
/// row/column where the edit begins; `count` (≥ 1 to have any effect) is how
/// many lines are inserted or deleted.
///
/// Coordinates are 1-based to match the public A1 surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralEdit {
    /// Insert `count` blank rows *before* row `at`; every row ≥ `at` slides down.
    InsertRows {
        /// 1-based row the blanks are inserted before.
        at: u32,
        /// How many blank rows to insert.
        count: u32,
    },
    /// Delete `count` rows starting at row `at`; every row after the band slides up.
    DeleteRows {
        /// 1-based first deleted row.
        at: u32,
        /// How many rows to delete.
        count: u32,
    },
    /// Insert `count` blank columns *before* column `at`; columns ≥ `at` slide right.
    InsertCols {
        /// 1-based column the blanks are inserted before.
        at: u32,
        /// How many blank columns to insert.
        count: u32,
    },
    /// Delete `count` columns starting at column `at`; columns after slide left.
    DeleteCols {
        /// 1-based first deleted column.
        at: u32,
        /// How many columns to delete.
        count: u32,
    },
}

/// Adjust one 1-based coordinate for an **insert** of `count` lines before `at`.
/// Lines at or after the insertion point slide outward; earlier lines are
/// unchanged. Saturates at `u32::MAX` rather than wrapping (a reference pushed
/// past the end of the grid by an absurd insert is clamped, never corrupted).
fn insert_coord(v: u32, at: u32, count: u32) -> u32 {
    if v >= at {
        v.saturating_add(count)
    } else {
        v
    }
}

/// Adjust one 1-based coordinate for a **delete** of the band `[at, at+count)`.
/// Returns `None` if the coordinate is *inside* the deleted band (the cell it
/// names no longer exists); coordinates after the band slide inward.
fn delete_coord(v: u32, at: u32, count: u32) -> Option<u32> {
    let band_end = at.saturating_add(count); // exclusive
    if v < at {
        Some(v) // before the band — unchanged
    } else if v < band_end {
        None // inside the band — destroyed
    } else {
        Some(v - count) // after the band — slides up/left
    }
}

impl CellAddress {
    /// Where this address moves to under a [`StructuralEdit`], or `None` if it
    /// sat on a deleted row/column (the reference becomes `#REF!`).
    ///
    /// Absolute flags are **preserved but not exempted** — a structural edit
    /// moves the grid itself, so even `$A$1` shifts (see the module docs). Use
    /// [`CellAddress::shift`] for copy/paste arithmetic, where absolute flags
    /// *do* pin the reference.
    pub fn adjust(&self, edit: StructuralEdit) -> Option<CellAddress> {
        let mut out = *self;
        match edit {
            StructuralEdit::InsertRows { at, count } => out.row = insert_coord(self.row, at, count),
            StructuralEdit::DeleteRows { at, count } => out.row = delete_coord(self.row, at, count)?,
            StructuralEdit::InsertCols { at, count } => out.col = insert_coord(self.col, at, count),
            StructuralEdit::DeleteCols { at, count } => out.col = delete_coord(self.col, at, count)?,
        }
        Some(out)
    }
}

/// Adjust an inclusive `[start, end]` interval along the edited axis.
/// Returns `None` only when the whole interval lay inside a deleted band (the
/// range collapses to nothing → `#REF!`). Partial overlaps clamp:
/// - **insert** before the interval moves it; inside it grows it; after leaves it.
/// - **delete** before shifts it; overlapping shrinks it to the surviving part.
fn adjust_interval(
    start: u32,
    end: u32,
    at: u32,
    count: u32,
    insert: bool,
) -> Option<(u32, u32)> {
    if insert {
        // Each endpoint follows the per-coordinate insert rule independently:
        // start stays unless the blanks land at/above it, end slides if the
        // blanks land at/above it — so a mid-range insert grows the range.
        Some((insert_coord(start, at, count), insert_coord(end, at, count)))
    } else {
        let band_end = at.saturating_add(count); // exclusive
        // Clamp the start to the first surviving line at/after it, and the end to
        // the last surviving line at/before it.
        let new_start = if start < at {
            start
        } else if start < band_end {
            at // first row that survives *is* the post-deletion coord `at`
        } else {
            start - count
        };
        let new_end = if end < at {
            end
        } else if end < band_end {
            at.checked_sub(1)? // last surviving line before the band
        } else {
            end - count
        };
        // If the surviving start overtook the surviving end, the whole interval
        // was deleted.
        if new_start > new_end {
            None
        } else {
            Some((new_start, new_end))
        }
    }
}

impl CellRange {
    /// Where this range moves to under a [`StructuralEdit`], or `None` if the
    /// range was entirely deleted (→ `#REF!`). The unedited axis is untouched;
    /// the edited axis follows [`adjust_interval`]'s grow/move/shrink rules.
    /// Absolute flags on the corners are preserved.
    pub fn adjust(&self, edit: StructuralEdit) -> Option<CellRange> {
        let mut start = self.start;
        let mut end = self.end;
        match edit {
            StructuralEdit::InsertRows { at, count } | StructuralEdit::DeleteRows { at, count } => {
                let insert = matches!(edit, StructuralEdit::InsertRows { .. });
                let (s, e) = adjust_interval(self.start.row, self.end.row, at, count, insert)?;
                start.row = s;
                end.row = e;
            }
            StructuralEdit::InsertCols { at, count } | StructuralEdit::DeleteCols { at, count } => {
                let insert = matches!(edit, StructuralEdit::InsertCols { .. });
                let (s, e) = adjust_interval(self.start.col, self.end.col, at, count, insert)?;
                start.col = s;
                end.col = e;
            }
        }
        Some(CellRange { start, end })
    }
}

impl FormulaAst {
    /// Rewrite every reference in this formula for a [`StructuralEdit`], so the
    /// formula keeps naming the same logical cells after rows/columns move.
    /// References (or ranges) that fell on deleted lines become the `#REF!`
    /// error literal, which then propagates through evaluation like any error.
    ///
    /// Pure: returns a new tree, leaving `self` untouched.
    pub fn adjust(&self, edit: StructuralEdit) -> FormulaAst {
        match self {
            FormulaAst::Literal(v) => FormulaAst::Literal(v.clone()),
            // An unqualified reference points into this formula's own sheet — the
            // sheet being edited — so it shifts (or collapses to `#REF!` if its
            // band was deleted).
            FormulaAst::Ref { sheet: None, addr } => match addr.adjust(edit) {
                Some(a) => FormulaAst::cell(a),
                None => ref_error(),
            },
            FormulaAst::Range { sheet: None, range } => match range.adjust(edit) {
                Some(r) => FormulaAst::cell_range(r),
                None => ref_error(),
            },
            // A cross-sheet reference targets another sheet, so a structural edit
            // on this formula's sheet leaves it untouched. (Propagating an edit on
            // sheet S to inbound `S!`-qualified refs that live on *other* sheets is
            // a separate workbook-level pass, handled in a later slice.)
            FormulaAst::Ref {
                sheet: Some(s),
                addr,
            } => FormulaAst::sheet_cell(s.clone(), *addr),
            FormulaAst::Range {
                sheet: Some(s),
                range,
            } => FormulaAst::sheet_range(s.clone(), *range),
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.adjust(edit)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.adjust(edit)),
                rhs: Box::new(rhs.adjust(edit)),
            },
            FormulaAst::Percent(inner) => FormulaAst::Percent(Box::new(inner.adjust(edit))),
            FormulaAst::Call { name, args } => FormulaAst::Call {
                name: name.clone(),
                args: args.iter().map(|a| a.adjust(edit)).collect(),
            },
        }
    }

    /// Adjust this formula's references for a structural [`edit`] applied to a
    /// **specific** sheet, identified by `edited_name` and whether it is this
    /// formula's own (host) sheet (`edited_is_host`).
    ///
    /// A reference shifts (or collapses to `#REF!` if its band was deleted) **only
    /// if it points into the edited sheet** — that is, an *unqualified* ref when
    /// the formula's host sheet is the one edited, or a *qualified* ref whose name
    /// matches `edited_name`. Every other reference (an unqualified ref on a
    /// non-edited sheet, or a qualified ref to a different sheet) is left exactly
    /// as-is. This is what lets an edit on sheet `S` ripple into inbound `S!…`
    /// references that live on *other* sheets (the workbook walks every sheet and
    /// calls this with `edited_is_host = false` for the non-edited ones), while a
    /// formula's references into untouched sheets stay put.
    ///
    /// Pure: returns a new tree.
    pub fn adjust_for_sheet_edit(
        &self,
        edit: StructuralEdit,
        edited_is_host: bool,
        edited_name: &str,
    ) -> FormulaAst {
        match self {
            FormulaAst::Literal(_) => self.clone(),
            // Unqualified ref → points into the host sheet; shifts iff that sheet
            // is the one being edited.
            FormulaAst::Ref { sheet: None, addr } => {
                if edited_is_host {
                    match addr.adjust(edit) {
                        Some(a) => FormulaAst::cell(a),
                        None => ref_error(),
                    }
                } else {
                    self.clone()
                }
            }
            FormulaAst::Range { sheet: None, range } => {
                if edited_is_host {
                    match range.adjust(edit) {
                        Some(r) => FormulaAst::cell_range(r),
                        None => ref_error(),
                    }
                } else {
                    self.clone()
                }
            }
            // Qualified ref → points into the named sheet; shifts iff that name is
            // the edited sheet (a cross-sheet ref into `S` follows `S`'s edit).
            FormulaAst::Ref {
                sheet: Some(name),
                addr,
            } => {
                if name == edited_name {
                    match addr.adjust(edit) {
                        Some(a) => FormulaAst::sheet_cell(name.clone(), a),
                        None => ref_error(),
                    }
                } else {
                    self.clone()
                }
            }
            FormulaAst::Range {
                sheet: Some(name),
                range,
            } => {
                if name == edited_name {
                    match range.adjust(edit) {
                        Some(r) => FormulaAst::sheet_range(name.clone(), r),
                        None => ref_error(),
                    }
                } else {
                    self.clone()
                }
            }
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.adjust_for_sheet_edit(edit, edited_is_host, edited_name)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.adjust_for_sheet_edit(edit, edited_is_host, edited_name)),
                rhs: Box::new(rhs.adjust_for_sheet_edit(edit, edited_is_host, edited_name)),
            },
            FormulaAst::Percent(inner) => FormulaAst::Percent(Box::new(
                inner.adjust_for_sheet_edit(edit, edited_is_host, edited_name),
            )),
            FormulaAst::Call { name, args } => FormulaAst::Call {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| a.adjust_for_sheet_edit(edit, edited_is_host, edited_name))
                    .collect(),
            },
        }
    }
}

/// The `#REF!` error as a formula literal — what a reference to a deleted cell
/// collapses to.
fn ref_error() -> FormulaAst {
    FormulaAst::Literal(CellValue::Error(SpreadsheetError::Ref))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: &str) -> CellAddress {
        CellAddress::parse(s).unwrap()
    }
    fn r(a1: &str, a2: &str) -> CellRange {
        CellRange::new(a(a1), a(a2))
    }

    // ── adjust_for_sheet_edit (cross-sheet structural propagation) ──
    #[test]
    fn adjust_for_sheet_edit_targets_only_refs_into_the_edited_sheet() {
        use crate::parser::parse;
        let e = StructuralEdit::InsertRows { at: 1, count: 1 };
        // On the edited sheet itself (edited_is_host = true): the unqualified ref
        // shifts; a ref into another sheet ("Other") is left alone.
        let host = parse("=A1+Other!A1").unwrap();
        assert_eq!(
            host.adjust_for_sheet_edit(e, true, "Summary").to_formula_string(),
            "(A2+Other!A1)"
        );
        // On a DIFFERENT sheet (edited_is_host = false), only refs qualified with
        // the edited sheet's name shift; the formula's own (unqualified) ref stays.
        let inbound = parse("=A1+Summary!A1").unwrap();
        assert_eq!(
            inbound.adjust_for_sheet_edit(e, false, "Summary").to_formula_string(),
            "(A1+Summary!A2)"
        );
        // A qualified ref to a deleted band becomes #REF!.
        let del = StructuralEdit::DeleteRows { at: 1, count: 1 };
        assert_eq!(
            parse("=Summary!A1")
                .unwrap()
                .adjust_for_sheet_edit(del, false, "Summary")
                .to_formula_string(),
            "#REF!"
        );
    }

    // ── Address: insert rows ────────────────────────────────────────
    #[test]
    fn insert_rows_shifts_at_and_after() {
        let e = StructuralEdit::InsertRows { at: 3, count: 2 };
        assert_eq!(a("A2").adjust(e), Some(a("A2"))); // before — unchanged
        assert_eq!(a("A3").adjust(e), Some(a("A5"))); // at — slides down by 2
        assert_eq!(a("A10").adjust(e), Some(a("A12"))); // after — slides down
        assert_eq!(a("B3").adjust(e), Some(a("B5"))); // other columns ride along
    }

    #[test]
    fn insert_rows_shifts_absolute_refs_too() {
        let e = StructuralEdit::InsertRows { at: 1, count: 1 };
        // $A$1 is absolute against copy/paste, but a structural insert still
        // moves it: the grid itself shifted.
        let moved = a("$A$1").adjust(e).unwrap();
        assert_eq!(moved.row, 2);
        assert!(moved.absolute_row && moved.absolute_col); // flags preserved
        assert_eq!(moved.to_a1(), "$A$2");
    }

    // ── Address: delete rows ────────────────────────────────────────
    #[test]
    fn delete_rows_destroys_band_and_shifts_after() {
        let e = StructuralEdit::DeleteRows { at: 3, count: 2 }; // delete rows 3,4
        assert_eq!(a("A2").adjust(e), Some(a("A2"))); // before — unchanged
        assert_eq!(a("A3").adjust(e), None); // in band — #REF!
        assert_eq!(a("A4").adjust(e), None); // in band — #REF!
        assert_eq!(a("A5").adjust(e), Some(a("A3"))); // after — slides up by 2
    }

    // ── Address: columns mirror rows ────────────────────────────────
    #[test]
    fn insert_and_delete_columns() {
        let ins = StructuralEdit::InsertCols { at: 2, count: 1 }; // before B
        assert_eq!(a("A1").adjust(ins), Some(a("A1"))); // col A unchanged
        assert_eq!(a("B1").adjust(ins), Some(a("C1"))); // B → C
        let del = StructuralEdit::DeleteCols { at: 2, count: 1 }; // delete B
        assert_eq!(a("B1").adjust(del), None); // #REF!
        assert_eq!(a("C1").adjust(del), Some(a("B1"))); // C → B
    }

    // ── Range: insert grows when inside, moves when before ──────────
    #[test]
    fn insert_inside_range_grows_it() {
        let e = StructuralEdit::InsertRows { at: 3, count: 1 };
        // A2:A5 with a row inserted at 3: start (2) stays, end (5) slides → A2:A6.
        assert_eq!(r("A2", "A5").adjust(e), Some(r("A2", "A6")));
    }

    #[test]
    fn insert_before_range_moves_it() {
        let e = StructuralEdit::InsertRows { at: 1, count: 2 };
        // A2:A5 with 2 rows inserted at the top → both ends slide → A4:A7.
        assert_eq!(r("A2", "A5").adjust(e), Some(r("A4", "A7")));
    }

    // ── Range: delete shrinks / shifts / destroys ───────────────────
    #[test]
    fn delete_inside_range_shrinks_it() {
        let e = StructuralEdit::DeleteRows { at: 3, count: 1 }; // delete row 3
        // A2:A5 loses one interior row → A2:A4.
        assert_eq!(r("A2", "A5").adjust(e), Some(r("A2", "A4")));
    }

    #[test]
    fn delete_overlapping_range_start_clamps() {
        let e = StructuralEdit::DeleteRows { at: 2, count: 2 }; // delete rows 2,3
        // A2:A5: rows 2,3 gone → surviving part is old 4,5, now at 2,3 → A2:A3.
        assert_eq!(r("A2", "A5").adjust(e), Some(r("A2", "A3")));
    }

    #[test]
    fn delete_whole_range_is_ref_error() {
        let e = StructuralEdit::DeleteRows { at: 2, count: 4 }; // delete rows 2..5
        assert_eq!(r("A2", "A5").adjust(e), None);
    }

    #[test]
    fn delete_after_range_shifts_nothing_on_the_range() {
        let e = StructuralEdit::DeleteRows { at: 10, count: 2 };
        assert_eq!(r("A2", "A5").adjust(e), Some(r("A2", "A5")));
    }

    // ── Formula AST rewriting ───────────────────────────────────────
    use crate::parser::parse;

    fn adjusted_formula(src: &str, edit: StructuralEdit) -> FormulaAst {
        parse(src).unwrap().adjust(edit)
    }

    #[test]
    fn formula_refs_follow_inserted_rows() {
        // =A1+A2 with a row inserted at 1 → =A2+A3 (both refs slide down).
        let ast = adjusted_formula("=A1+A2", StructuralEdit::InsertRows { at: 1, count: 1 });
        let want = parse("=A2+A3").unwrap();
        assert_eq!(ast, want);
    }

    #[test]
    fn formula_range_grows_under_interior_insert() {
        // =SUM(A1:A4) with a row inserted at 3 → =SUM(A1:A5).
        let ast = adjusted_formula("=SUM(A1:A4)", StructuralEdit::InsertRows { at: 3, count: 1 });
        let want = parse("=SUM(A1:A5)").unwrap();
        assert_eq!(ast, want);
    }

    #[test]
    fn deleted_ref_becomes_ref_error_and_survivors_shift() {
        // =A3+A5 with row 3 deleted → =#REF!+A4.
        let ast = adjusted_formula("=A3+A5", StructuralEdit::DeleteRows { at: 3, count: 1 });
        match ast {
            FormulaAst::Binary { lhs, rhs, .. } => {
                assert_eq!(*lhs, ref_error());
                assert_eq!(*rhs, FormulaAst::cell(a("A4")));
            }
            other => panic!("expected a binary op, got {other:?}"),
        }
    }

    #[test]
    fn deleted_range_becomes_ref_error_inside_call() {
        // =SUM(A2:A3) with rows 2,3 deleted → =SUM(#REF!).
        let ast = adjusted_formula("=SUM(A2:A3)", StructuralEdit::DeleteRows { at: 2, count: 2 });
        match ast {
            FormulaAst::Call { name, args } => {
                assert_eq!(name.to_ascii_uppercase(), "SUM");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], ref_error());
            }
            other => panic!("expected a call, got {other:?}"),
        }
    }

    #[test]
    fn literals_and_nested_nodes_are_preserved() {
        // A literal with no refs is unchanged; nested ops recurse.
        let e = StructuralEdit::InsertCols { at: 1, count: 1 };
        assert_eq!(adjusted_formula("=1+2", e), parse("=1+2").unwrap());
        // -A1% with a column inserted before A → -B1% .
        let ast = adjusted_formula("=-A1%", e);
        let want = parse("=-B1%").unwrap();
        assert_eq!(ast, want);
    }

    #[test]
    fn zero_count_edit_is_identity() {
        let e = StructuralEdit::InsertRows { at: 1, count: 0 };
        assert_eq!(a("A5").adjust(e), Some(a("A5")));
        assert_eq!(r("A1", "A4").adjust(e), Some(r("A1", "A4")));
    }
}
