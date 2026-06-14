//! The Workbook — the top-level container.
//!
//! Holds sheets, cells, the dependency graph, and the recalc epoch.
//! Phase 1 ships a minimal but complete engine: literal + formula
//! cells, dependency tracking, automatic-recalc-on-edit (the user
//! can also call `recalc_all` for a full sweep).

use std::collections::HashMap;

use crate::address::{CellAddress, SheetId};
use crate::cell::{Cell, CellContent, CellValue};
use crate::dag::DependencyGraph;
use crate::errors::SpreadsheetError;
use crate::parser::{parse, ParseError};
use crate::recalc::{collect_refs, evaluate};

/// Top-level container — one or more sheets plus the dependency
/// graph that spans them.
pub struct Workbook {
    /// Sheets in order. `SheetId(i)` indexes here.
    sheets: Vec<Sheet>,
    /// Sheet-name → id lookup.
    sheet_by_name: HashMap<String, SheetId>,
    /// Cross-sheet dependency graph.
    graph: DependencyGraph,
    /// Recalc epoch; bumped after every successful `recalc_all`.
    epoch: u64,
}

struct Sheet {
    name: String,
    cells: HashMap<CellAddress, Cell>,
}

impl Workbook {
    /// Construct an empty workbook with no sheets.
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            sheet_by_name: HashMap::new(),
            graph: DependencyGraph::new(),
            epoch: 0,
        }
    }

    /// Add a sheet. Sheet names must be unique within a workbook.
    pub fn add_sheet(&mut self, name: impl Into<String>) -> SheetId {
        let name = name.into();
        let id = SheetId(self.sheets.len() as u32);
        self.sheets.push(Sheet {
            name: name.clone(),
            cells: HashMap::new(),
        });
        self.sheet_by_name.insert(name, id);
        id
    }

    /// Number of sheets.
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// Look up a sheet by name.
    pub fn sheet_id(&self, name: &str) -> Option<SheetId> {
        self.sheet_by_name.get(name).copied()
    }

    /// Look up a sheet's name by id — the inverse of [`sheet_id`].
    /// Returns `None` if the id does not refer to an existing sheet.
    ///
    /// [`sheet_id`]: Workbook::sheet_id
    pub fn sheet_name(&self, sheet: SheetId) -> Option<&str> {
        self.sheets.get(sheet.0 as usize).map(|s| s.name.as_str())
    }

    /// Get the recalc epoch — bumped on every successful
    /// `recalc_all`.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Set a literal value (no formula). Updates the dependency
    /// graph (removes any prior edges from this cell) and triggers
    /// recalc of downstream cells.
    pub fn set_value(&mut self, sheet: SheetId, addr: CellAddress, value: CellValue) {
        let s = &mut self.sheets[sheet.0 as usize];
        s.cells.insert(addr, Cell::value(value));
        self.graph.remove((sheet, addr));
        self.recalc_dependents_of(sheet, addr);
    }

    /// Set a formula. Parses the text; on a parse error, stores the
    /// cell as `#NAME?` and returns the parse error.
    pub fn set_formula(
        &mut self,
        sheet: SheetId,
        addr: CellAddress,
        text: &str,
    ) -> Result<(), ParseError> {
        let ast = parse(text)?;
        let s = &mut self.sheets[sheet.0 as usize];
        s.cells.insert(
            addr,
            Cell {
                content: CellContent::Formula {
                    ast: ast.clone(),
                    text: text.to_string(),
                    cached: None,
                },
            },
        );
        // Update dependency edges.
        let mut refs = Vec::new();
        collect_refs(&ast, sheet, &mut refs);
        self.graph.set_dependencies((sheet, addr), refs);
        // Evaluate the new formula and recalc downstream.
        self.recalc_dependents_of(sheet, addr);
        self.evaluate_cell(sheet, addr);
        Ok(())
    }

    /// Mark a cell empty.
    pub fn clear_cell(&mut self, sheet: SheetId, addr: CellAddress) {
        let s = &mut self.sheets[sheet.0 as usize];
        s.cells.remove(&addr);
        self.graph.remove((sheet, addr));
        self.recalc_dependents_of(sheet, addr);
    }

    /// Read the current value of a cell. Returns `None` if the cell
    /// has never been touched; returns `Some(Empty)` for an
    /// explicitly cleared cell.
    pub fn get_value(&self, sheet: SheetId, addr: CellAddress) -> Option<CellValue> {
        let s = self.sheets.get(sheet.0 as usize)?;
        s.cells.get(&addr).map(|c| c.current_value())
    }

    /// Read a cell's stored value, falling back to `Empty` for
    /// missing cells. This is the lookup-callback view used by
    /// formula evaluation.
    pub fn cell_value(&self, sheet: SheetId, addr: CellAddress) -> CellValue {
        self.get_value(sheet, addr).unwrap_or(CellValue::Empty)
    }

    /// Recalculate every formula cell. Bumps the epoch on success.
    pub fn recalc_all(&mut self) {
        // Build a set of all formula cells.
        let mut all: std::collections::HashSet<(SheetId, CellAddress)> =
            std::collections::HashSet::new();
        for (i, s) in self.sheets.iter().enumerate() {
            let sheet = SheetId(i as u32);
            for (addr, cell) in &s.cells {
                if cell.is_formula() {
                    all.insert((sheet, *addr));
                }
            }
        }
        let (order, cycles) = self.graph.topological_order(&all);
        for (sheet, addr) in order {
            self.evaluate_cell(sheet, addr);
        }
        for (sheet, addr) in cycles {
            self.set_cached(sheet, addr, CellValue::Error(SpreadsheetError::Ref));
        }
        self.epoch = self.epoch.wrapping_add(1);
    }

    // ----------------------------------------------------------------
    // Internal helpers
    // ----------------------------------------------------------------

    fn recalc_dependents_of(&mut self, sheet: SheetId, addr: CellAddress) {
        let dirty = self.graph.transitive_dependents((sheet, addr));
        if dirty.is_empty() {
            return;
        }
        let (order, cycles) = self.graph.topological_order(&dirty);
        for (sheet, addr) in order {
            self.evaluate_cell(sheet, addr);
        }
        for (sheet, addr) in cycles {
            self.set_cached(sheet, addr, CellValue::Error(SpreadsheetError::Ref));
        }
    }

    fn evaluate_cell(&mut self, sheet: SheetId, addr: CellAddress) {
        // Clone the AST out to avoid holding a borrow during evaluate.
        let ast = {
            let s = &self.sheets[sheet.0 as usize];
            match s.cells.get(&addr) {
                Some(Cell {
                    content: CellContent::Formula { ast, .. },
                }) => Some(ast.clone()),
                _ => None,
            }
        };
        let Some(ast) = ast else { return };
        // Evaluate against a read-only *borrow* of the cell storage.
        // `lookup` resolves each referenced cell on demand straight out
        // of `self.sheets`; nothing is cloned up front. (An earlier
        // version snapshotted every cell of every sheet into a fresh
        // HashMap on each call — O(N) allocation per cell, so O(N²) to
        // recalc N cells, which made a few thousand interdependent
        // formulas hang the host. `evaluate` returns an owned value, so
        // the borrow ends before we take `&mut self` to cache below.)
        let result = {
            let sheets = &self.sheets;
            let lookup = |sid: SheetId, a: CellAddress| {
                sheets
                    .get(sid.0 as usize)
                    .and_then(|s| s.cells.get(&a))
                    .map(|c| c.current_value())
                    .unwrap_or(CellValue::Empty)
            };
            match evaluate(&ast, sheet, &lookup) {
                Ok(v) => v,
                Err(e) => CellValue::Error(e),
            }
        };
        self.set_cached(sheet, addr, result);
    }

    fn set_cached(&mut self, sheet: SheetId, addr: CellAddress, value: CellValue) {
        let s = &mut self.sheets[sheet.0 as usize];
        if let Some(cell) = s.cells.get_mut(&addr) {
            if let CellContent::Formula { cached, .. } = &mut cell.content {
                *cached = Some(value);
            }
        }
    }
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(row: u32, col: u32) -> CellAddress {
        CellAddress::new(row, col)
    }

    #[test]
    fn empty_workbook_no_sheets() {
        let wb = Workbook::new();
        assert_eq!(wb.sheet_count(), 0);
        assert_eq!(wb.epoch(), 0);
    }

    #[test]
    fn add_sheet_assigns_id() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let s2 = wb.add_sheet("Sheet2");
        assert_ne!(s1, s2);
        assert_eq!(wb.sheet_id("Sheet1"), Some(s1));
    }

    #[test]
    fn sheet_name_is_inverse_of_sheet_id() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let s2 = wb.add_sheet("Budget");
        assert_eq!(wb.sheet_name(s1), Some("Sheet1"));
        assert_eq!(wb.sheet_name(s2), Some("Budget"));
        // Round-trips with sheet_id.
        assert_eq!(wb.sheet_id(wb.sheet_name(s2).unwrap()), Some(s2));
        // Unknown id → None.
        assert_eq!(wb.sheet_name(SheetId(99)), None);
    }

    #[test]
    fn set_value_then_read() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(42.0));
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(42.0)));
    }

    #[test]
    fn formula_evaluates_on_set() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(2.0));
        wb.set_value(s, cell(1, 2), CellValue::Number(3.0));
        wb.set_formula(s, cell(1, 3), "=A1+B1").unwrap();
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(5.0)));
    }

    #[test]
    fn dependent_cell_updates_on_input_change() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(2.0));
        wb.set_formula(s, cell(1, 2), "=A1*10").unwrap();
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(20.0)));
        // Change the input.
        wb.set_value(s, cell(1, 1), CellValue::Number(7.0));
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(70.0)));
    }

    #[test]
    fn chained_dependencies_recalc_in_order() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_formula(s, cell(1, 2), "=A1+1").unwrap();
        wb.set_formula(s, cell(1, 3), "=B1+1").unwrap();
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(3.0)));
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0));
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(12.0)));
    }

    #[test]
    fn circular_reference_yields_ref_error() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_formula(s, cell(1, 1), "=B1+1").unwrap();
        wb.set_formula(s, cell(1, 2), "=A1+1").unwrap();
        wb.recalc_all();
        assert!(matches!(
            wb.get_value(s, cell(1, 1)),
            Some(CellValue::Error(SpreadsheetError::Ref))
        ));
        assert!(matches!(
            wb.get_value(s, cell(1, 2)),
            Some(CellValue::Error(SpreadsheetError::Ref))
        ));
    }

    #[test]
    fn function_call_over_range() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for i in 1..=5 {
            wb.set_value(s, cell(i, 1), CellValue::Number(i as f64));
        }
        wb.set_formula(s, cell(1, 2), "=SUM(A1:A5)").unwrap();
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(15.0)));
    }

    #[test]
    fn parse_error_surfaces() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        let err = wb.set_formula(s, cell(1, 1), "=1 + ").unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedEof));
    }

    #[test]
    fn lookup_returns_empty_for_unset_cell() {
        let wb = Workbook::new();
        let mut wb = wb;
        let s = wb.add_sheet("S");
        wb.set_formula(s, cell(1, 1), "=A99+1").unwrap();
        // A99 is empty → coerces to 0 → result is 1.
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
    }

    #[test]
    fn clear_cell_removes_value() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(5.0));
        wb.clear_cell(s, cell(1, 1));
        assert_eq!(wb.get_value(s, cell(1, 1)), None);
    }

    #[test]
    fn recalc_all_bumps_epoch() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_formula(s, cell(1, 2), "=A1*2").unwrap();
        let e0 = wb.epoch();
        wb.recalc_all();
        assert_eq!(wb.epoch(), e0 + 1);
    }

    #[test]
    fn iferror_catches_div_zero() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(0.0));
        wb.set_formula(s, cell(1, 2), "=IFERROR(1/A1, 0)").unwrap();
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(0.0)));
    }

    #[test]
    fn oversized_range_yields_ref_error_not_oom() {
        // A single typed formula naming ~17 billion cells must surface
        // #REF! rather than trying to allocate one value (and one
        // dependency-graph entry) per cell. set_formula returns Ok (the
        // formula parses fine); the error appears as the cell's value.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_formula(s, cell(1, 1), "=SUM(A1:XFD1048576)").unwrap();
        assert_eq!(
            wb.get_value(s, cell(1, 1)),
            Some(CellValue::Error(SpreadsheetError::Ref)),
        );
    }

    #[test]
    fn long_formula_chain_recalcs_without_quadratic_blowup() {
        // A1=1, A2=A1+1, …, A_N=A_{N-1}+1. Each evaluate borrows the
        // cell storage instead of cloning the whole workbook, so this
        // is linear, not quadratic; and the topo order comes from the
        // iterative Tarjan, so the deep chain doesn't overflow.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        const N: u32 = 2_000;
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        for r in 2..=N {
            wb.set_formula(s, cell(r, 1), &format!("=A{}+1", r - 1)).unwrap();
        }
        assert_eq!(wb.get_value(s, cell(N, 1)), Some(CellValue::Number(N as f64)));
    }
}
