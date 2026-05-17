//! The main formula engine — storage, parsing, dependency tracking, and
//! recalculation.
//!
//! [`FormulaEngine`] is the central struct.  Think of it as an in-memory
//! spreadsheet with a grid of cells.  Each cell stores:
//!
//! - The raw string the user typed (e.g. `"42"` or `"=A1*2"`).
//! - The current computed value (a [`CellValue`]).
//! - A "dirty" flag that indicates the value needs recalculation.
//!
//! The engine maintains a [`DependencyGraph`] so that when cell A1 changes,
//! all cells that reference A1 are automatically marked dirty.  Calling
//! [`FormulaEngine::recalculate`] then evaluates them in the correct order.
//!
//! # Lifecycle
//!
//! ```text
//! 1.  engine.set_raw(addr, raw_string)    → marks the cell dirty
//! 2.  engine.recalculate()               → evaluates all dirty cells in
//!                                           topological order
//! 3.  engine.get_display(&addr)          → returns the computed display string
//! ```

use std::collections::{HashMap, HashSet};

use crate::addr::CellAddr;
use crate::eval::{collect_refs, eval};
use crate::graph::DependencyGraph;
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::value::CellValue;
use crate::FormulaError;

/// Storage for a single cell.
#[derive(Debug, Clone)]
struct Cell {
    /// The raw string the user typed.
    raw: String,
    /// The current computed value (may be stale if dirty).
    value: CellValue,
    /// True when the cell needs to be recalculated.
    dirty: bool,
}

impl Cell {
    fn new(raw: String) -> Self {
        Cell {
            raw,
            value: CellValue::Empty,
            dirty: true,
        }
    }
}

/// The formula engine.
///
/// Create one with [`FormulaEngine::new`], populate cells with
/// [`FormulaEngine::set_raw`], then call [`FormulaEngine::recalculate`]
/// before reading values.
#[derive(Debug, Default)]
pub struct FormulaEngine {
    cells: HashMap<CellAddr, Cell>,
    graph: DependencyGraph,
    dirty: HashSet<CellAddr>,
}

impl FormulaEngine {
    /// Create a new, empty engine.
    pub fn new() -> Self {
        FormulaEngine::default()
    }

    /// Set the raw content of a cell.
    ///
    /// - If `raw` starts with `'='`, it is treated as a formula.
    /// - Otherwise it is a literal: a parseable `f64` becomes `Number`,
    ///   anything else becomes `Text`.
    ///
    /// The cell is marked dirty; call [`recalculate`](Self::recalculate)
    /// afterwards to update computed values.
    pub fn set_raw(&mut self, addr: CellAddr, raw: String) {
        // Update the dependency graph based on the new formula.
        if let Some(formula) = raw.strip_prefix('=') {
            // Try to extract cell references from the formula.
            if let Ok(tokens) = tokenize(formula) {
                if let Ok(expr) = parse(tokens) {
                    let refs = collect_refs(&expr);
                    self.graph.set_deps(&addr, &refs);
                } else {
                    self.graph.clear_deps(&addr);
                }
            } else {
                self.graph.clear_deps(&addr);
            }
        } else {
            // Literals have no dependencies.
            self.graph.clear_deps(&addr);
        }

        // Mark this cell and all cells that depend on it as dirty.
        let dependents = self.graph.transitive_dependents(&addr);
        self.dirty.insert(addr.clone());
        self.dirty.extend(dependents);

        // Store the raw value.
        self.cells.insert(addr, Cell::new(raw));
    }

    /// Get the display string for a cell (what the user sees in the cell).
    ///
    /// Returns an empty string for cells that have never been set.
    /// If `recalculate` has not been called since the last `set_raw`, the
    /// returned value may be stale.
    pub fn get_display(&self, addr: &CellAddr) -> String {
        match self.cells.get(addr) {
            None => String::new(),
            Some(cell) => cell.value.display_string(),
        }
    }

    /// Get the formula string for a cell (what appears in the formula bar).
    ///
    /// For formulas this is the original `"=..."` string.  For literals it
    /// is the raw string.  For unset cells it is `""`.
    pub fn get_formula(&self, addr: &CellAddr) -> String {
        match self.cells.get(addr) {
            None => String::new(),
            Some(cell) => cell.raw.clone(),
        }
    }

    /// Recalculate all dirty cells in dependency order.
    ///
    /// After this call, all dirty cells will have an up-to-date
    /// [`CellValue`].  Cells in a circular dependency are assigned
    /// `CellValue::Error(FormulaError::Circ)`.
    pub fn recalculate(&mut self) {
        if self.dirty.is_empty() {
            return;
        }

        // Step 1: topological sort of dirty cells.
        let (ordered, cycle_members) = self.graph.topological_sort(&self.dirty);

        // Step 2: mark cycle members immediately.
        for addr in &cycle_members {
            if let Some(cell) = self.cells.get_mut(addr) {
                cell.value = CellValue::Error(FormulaError::Circ);
                cell.dirty = false;
            }
        }

        // Step 3: evaluate in topological order.
        for addr in &ordered {
            let raw = match self.cells.get(addr) {
                Some(cell) => cell.raw.clone(),
                None => continue, // cell was in dirty set but not in storage (edge case)
            };

            let new_value = self.evaluate_raw(&raw);

            if let Some(cell) = self.cells.get_mut(addr) {
                cell.value = new_value;
                cell.dirty = false;
            }
        }

        // Clear the dirty set.
        self.dirty.clear();
    }

    // ── Private helpers ───────────────────────────────────────────────────

    /// Evaluate a raw cell string and return the computed value.
    ///
    /// For literals: parse as f64 → Number, else → Text.
    /// For formulas: lex, parse, evaluate with current cell values.
    fn evaluate_raw(&self, raw: &str) -> CellValue {
        if let Some(formula) = raw.strip_prefix('=') {
            // Formula path: tokenize → parse → eval.
            let tokens = match tokenize(formula) {
                Ok(t) => t,
                Err(_) => return CellValue::Error(FormulaError::Parse),
            };
            let expr = match parse(tokens) {
                Ok(e) => e,
                Err(_) => return CellValue::Error(FormulaError::Parse),
            };
            // The lookup function reads from the already-computed cell values.
            // Because we evaluate in topological order, any cell we reference
            // should already have its value updated.
            let lookup = |addr: &CellAddr| -> CellValue {
                self.cells
                    .get(addr)
                    .map(|c| c.value.clone())
                    .unwrap_or(CellValue::Empty)
            };
            eval(&expr, &lookup)
        } else if raw.is_empty() {
            CellValue::Empty
        } else if let Ok(n) = raw.parse::<f64>() {
            CellValue::Number(n)
        } else {
            CellValue::Text(raw.to_string())
        }
    }

    /// Convenience: parse a cell address string.
    pub fn cell_addr(s: &str) -> Result<CellAddr, FormulaError> {
        CellAddr::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> CellAddr {
        CellAddr::parse(s).unwrap()
    }

    // ── Literal cells ─────────────────────────────────────────────────────

    #[test]
    fn test_literal_number() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "42".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "42");
    }

    #[test]
    fn test_literal_text() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "Hello".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "Hello");
    }

    #[test]
    fn test_empty_cell_display() {
        let e = FormulaEngine::new();
        assert_eq!(e.get_display(&addr("A1")), "");
    }

    // ── Basic arithmetic formulas ─────────────────────────────────────────

    #[test]
    fn test_formula_addition() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=1+2".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "3");
    }

    #[test]
    fn test_formula_subtraction() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=10-3".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "7");
    }

    #[test]
    fn test_formula_multiplication() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=3*4".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "12");
    }

    #[test]
    fn test_formula_division() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=10/4".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "2.5");
    }

    #[test]
    fn test_division_by_zero() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=1/0".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "#DIV/0!");
    }

    #[test]
    fn test_negative_literal() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=-5".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "-5");
    }

    #[test]
    fn test_nested_parens() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=(1+2)*3".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "9");
    }

    // ── Cell references ───────────────────────────────────────────────────

    #[test]
    fn test_cell_reference() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "5".to_string());
        e.set_raw(addr("B1"), "=A1*2".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "10");
    }

    #[test]
    fn test_chain_reference() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "2".to_string());
        e.set_raw(addr("B1"), "=A1+1".to_string());
        e.set_raw(addr("C1"), "=B1+1".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("C1")), "4");
    }

    #[test]
    fn test_update_propagation() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "5".to_string());
        e.set_raw(addr("B1"), "=A1*2".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "10");

        // Update A1 — B1 should automatically become dirty.
        e.set_raw(addr("A1"), "10".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "20");
    }

    // ── Built-in functions ────────────────────────────────────────────────

    #[test]
    fn test_sum_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "1".to_string());
        e.set_raw(addr("A2"), "2".to_string());
        e.set_raw(addr("A3"), "3".to_string());
        e.set_raw(addr("B1"), "=SUM(A1,A2,A3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "6");
    }

    #[test]
    fn test_sum_range() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "1".to_string());
        e.set_raw(addr("A2"), "2".to_string());
        e.set_raw(addr("A3"), "3".to_string());
        e.set_raw(addr("B1"), "=SUM(A1:A3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "6");
    }

    #[test]
    fn test_avg_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=AVG(1,2,3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "2");
    }

    #[test]
    fn test_count_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "1".to_string());
        e.set_raw(addr("A2"), "hello".to_string());
        e.set_raw(addr("A3"), "3".to_string());
        e.set_raw(addr("B1"), "=COUNT(A1:A3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("B1")), "3");
    }

    #[test]
    fn test_max_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=MAX(3,1,4,1,5)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "5");
    }

    #[test]
    fn test_min_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=MIN(3,1,4,1,5)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "1");
    }

    #[test]
    fn test_if_true() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=IF(1,2,3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "2");
    }

    #[test]
    fn test_if_false() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=IF(0,2,3)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "3");
    }

    // ── Error cases ───────────────────────────────────────────────────────

    #[test]
    fn test_circular_reference() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=B1".to_string());
        e.set_raw(addr("B1"), "=A1".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "#CIRC");
        assert_eq!(e.get_display(&addr("B1")), "#CIRC");
    }

    #[test]
    fn test_error_propagation() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=1/0".to_string());
        e.set_raw(addr("B1"), "=A1+1".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "#DIV/0!");
        assert_eq!(e.get_display(&addr("B1")), "#DIV/0!");
    }

    #[test]
    fn test_unknown_function() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=FOO(1)".to_string());
        e.recalculate();
        assert_eq!(e.get_display(&addr("A1")), "#NAME?");
    }

    // ── Formula bar ───────────────────────────────────────────────────────

    #[test]
    fn test_get_formula_returns_raw() {
        let mut e = FormulaEngine::new();
        e.set_raw(addr("A1"), "=A1+1".to_string());
        assert_eq!(e.get_formula(&addr("A1")), "=A1+1");
    }

    // ── Convenience ───────────────────────────────────────────────────────

    #[test]
    fn test_cell_addr_convenience() {
        let addr = FormulaEngine::cell_addr("B5").unwrap();
        assert_eq!(addr.col(), 1);
        assert_eq!(addr.row(), 5);
    }
}
