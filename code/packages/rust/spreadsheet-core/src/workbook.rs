//! The Workbook — the top-level container.
//!
//! Holds sheets, cells, the dependency graph, and the recalc epoch.
//! Phase 1 ships a minimal but complete engine: literal + formula
//! cells, dependency tracking, automatic-recalc-on-edit (the user
//! can also call `recalc_all` for a full sweep).

use std::collections::{HashMap, HashSet, VecDeque};

use crate::address::{CellAddress, SheetId};
use crate::cell::{Cell, CellContent, CellValue};
use crate::dag::DependencyGraph;
use crate::edit::StructuralEdit;
use crate::errors::SpreadsheetError;
use crate::parser::{parse, ParseError};
use crate::recalc::{collect_refs, evaluate};
use crate::viewport::{ChangeSet, UsedRange, Window, CHANGELOG_RETAIN, MAX_WINDOW_CELLS};

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
    /// Per-edit revision clock for viewport diffing. Unlike `epoch` (which only
    /// advances on a full `recalc_all` sweep), `revision` advances once per
    /// mutation (`set_value` / `set_formula` / `clear_cell`), so a virtualized
    /// host can ask "what changed since the revision I last rendered?".
    revision: u64,
    /// Bounded change log: `(revision, sheet, addr)` for every cell whose value
    /// was (re)written, newest at the back. Pruned to `CHANGELOG_RETAIN`.
    changes: VecDeque<(u64, SheetId, CellAddress)>,
    /// Highest revision whose entries have been dropped from `changes`. A
    /// `changed_since(r)` with `r < dropped_revision` can't prove completeness
    /// and must answer `Stale`.
    dropped_revision: u64,
}

struct Sheet {
    name: String,
    cells: HashMap<CellAddress, Cell>,
    /// Per-cell display format codes (Excel-style, e.g. `"#,##0.00"`,
    /// `"yyyy-mm-dd"`). Stored separately from cell content because a cell can
    /// be formatted while empty, and the format outlives content edits — exactly
    /// as in a real spreadsheet. Applied to the computed value by [`get_display`].
    ///
    /// [`get_display`]: Workbook::get_display
    formats: HashMap<CellAddress, String>,
}

impl Workbook {
    /// Construct an empty workbook with no sheets.
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            sheet_by_name: HashMap::new(),
            graph: DependencyGraph::new(),
            epoch: 0,
            revision: 0,
            changes: VecDeque::new(),
            dropped_revision: 0,
        }
    }

    /// Add a sheet. Sheet names must be unique within a workbook.
    pub fn add_sheet(&mut self, name: impl Into<String>) -> SheetId {
        let name = name.into();
        let id = SheetId(self.sheets.len() as u32);
        self.sheets.push(Sheet {
            name: name.clone(),
            cells: HashMap::new(),
            formats: HashMap::new(),
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

    /// The per-edit revision clock, advanced once per mutation. A virtualized
    /// host snapshots this, then later calls [`changed_since`](Self::changed_since)
    /// with it to learn which cells changed in between.
    pub fn current_revision(&self) -> u64 {
        self.revision
    }

    /// Read a **dense** rectangle of computed values for the inclusive,
    /// 1-based window `(row0..=row1, col0..=col1)`, row-major. Empty cells are
    /// returned as [`CellValue::Empty`] so a host can index the result directly
    /// and render a solid grid. This is `O(window)` — it looks each address up
    /// in the sparse store — never `O(sheet)`.
    ///
    /// Errors with [`SpreadsheetError::Ref`] if the rectangle is inverted, the
    /// sheet id is unknown, or the window exceeds
    /// [`MAX_WINDOW_CELLS`](crate::viewport::MAX_WINDOW_CELLS) (the screen-scale
    /// safety cap — a host clamps to the visible window well below it).
    pub fn get_window(
        &self,
        sheet: SheetId,
        row0: u32,
        col0: u32,
        row1: u32,
        col1: u32,
    ) -> Result<Window, SpreadsheetError> {
        // Coordinates are 1-based (A1 = row 1, col 1), so a 0 is out of contract
        // — and rejecting it also rules out the `row0 = 0` case that would let
        // the span computation below span the full u32 range.
        if row0 == 0 || col0 == 0 || row1 < row0 || col1 < col0 {
            return Err(SpreadsheetError::Ref);
        }
        // Compute the span in u64. The operands MUST be widened to u64 *before*
        // the `+ 1`: doing `(row1 - row0 + 1)` in u32 first overflows when
        // `row1 - row0 == u32::MAX` (e.g. row0=1, row1=u32::MAX), wrapping to a
        // bogus small count that would slip past the MAX_WINDOW_CELLS cap and
        // send the loop over the entire u32 range — an OOM DoS. Widening first
        // keeps the true count, so checked_mul + the cap reject it.
        let rows = (row1 as u64 - row0 as u64) + 1;
        let cols = (col1 as u64 - col0 as u64) + 1;
        // `rows * cols` can still overflow u64 for a full-sheet request, so use
        // checked_mul: an overflow is by definition past the cap.
        match rows.checked_mul(cols) {
            Some(n) if n <= MAX_WINDOW_CELLS => {}
            _ => return Err(SpreadsheetError::Ref),
        }
        let s = self.sheets.get(sheet.0 as usize).ok_or(SpreadsheetError::Ref)?;
        let mut values = Vec::with_capacity((rows * cols) as usize);
        for r in row0..=row1 {
            for c in col0..=col1 {
                let v = s
                    .cells
                    .get(&CellAddress::new(r, c))
                    .map(|cell| cell.current_value())
                    .unwrap_or(CellValue::Empty);
                values.push(v);
            }
        }
        Ok(Window {
            row0,
            col0,
            rows: rows as u32,
            cols: cols as u32,
            values,
        })
    }

    /// The bounding box of all materialised, non-empty cells on `sheet`, or
    /// `None` if the sheet is empty. 1-based inclusive. A host uses this to size
    /// its scrollable area to the data. `O(materialised cells)`.
    pub fn used_range(&self, sheet: SheetId) -> Option<UsedRange> {
        let s = self.sheets.get(sheet.0 as usize)?;
        let mut range: Option<UsedRange> = None;
        for (addr, cell) in &s.cells {
            if cell.current_value().is_empty() {
                continue; // a present-but-empty cell doesn't extend the extent
            }
            match &mut range {
                None => {
                    range = Some(UsedRange {
                        min_row: addr.row,
                        min_col: addr.col,
                        max_row: addr.row,
                        max_col: addr.col,
                    })
                }
                Some(r) => {
                    r.min_row = r.min_row.min(addr.row);
                    r.min_col = r.min_col.min(addr.col);
                    r.max_row = r.max_row.max(addr.row);
                    r.max_col = r.max_col.max(addr.col);
                }
            }
        }
        range
    }

    /// Which cells on `sheet` changed strictly after `since_revision`.
    ///
    /// Returns [`ChangeSet::Delta`] with a deduped address list when the log
    /// still covers `since_revision`, or [`ChangeSet::Stale`] when the query
    /// reaches back before the retained window — in which case the host must
    /// re-read its whole visible window rather than risk missing a change.
    pub fn changed_since(&self, sheet: SheetId, since_revision: u64) -> ChangeSet {
        let current = self.revision;
        // If anything at or before `since_revision` was pruned, a change in
        // (since_revision, dropped_revision] may be gone — answer Stale. (When
        // since >= dropped_revision, every change after it is still retained.)
        if since_revision < self.dropped_revision {
            return ChangeSet::Stale {
                current_revision: current,
            };
        }
        let mut seen = HashSet::new();
        let mut changed = Vec::new();
        for (rev, sid, addr) in &self.changes {
            if *rev > since_revision && *sid == sheet && seen.insert(*addr) {
                changed.push(*addr);
            }
        }
        ChangeSet::Delta {
            current_revision: current,
            changed,
        }
    }

    /// Set a literal value (no formula). Updates the dependency
    /// graph (removes any prior edges from this cell) and triggers
    /// recalc of downstream cells.
    pub fn set_value(&mut self, sheet: SheetId, addr: CellAddress, value: CellValue) {
        self.revision = self.revision.wrapping_add(1);
        let s = &mut self.sheets[sheet.0 as usize];
        s.cells.insert(addr, Cell::value(value));
        self.graph.remove((sheet, addr));
        self.log_change(sheet, addr); // the literal itself changed
        self.recalc_dependents_of(sheet, addr); // dependents log via set_cached
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
        self.revision = self.revision.wrapping_add(1);
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
        self.revision = self.revision.wrapping_add(1);
        let s = &mut self.sheets[sheet.0 as usize];
        s.cells.remove(&addr);
        self.graph.remove((sheet, addr));
        self.log_change(sheet, addr); // the cell became empty
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

    // ----------------------------------------------------------------
    // Cell formats (display)
    // ----------------------------------------------------------------
    //
    // A format is an Excel-style code (`"#,##0.00"`, `"0%"`, `"yyyy-mm-dd"`,
    // `"h:mm AM/PM"`) that decides how a cell's *computed value* reads — never
    // what it is. The number/date formatting itself lives in `number-format-core`
    // (a Layer-1 core, like the math cores the formula engine dispatches to); the
    // engine just stores the per-cell code and applies it on display.

    /// Set a cell's display format code. An empty code clears the format (the
    /// cell falls back to `General`). Logs a change so a viewport sees the
    /// re-display, but does not touch the value or trigger recalc.
    pub fn set_format(&mut self, sheet: SheetId, addr: CellAddress, code: &str) {
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return;
        };
        self.revision = self.revision.wrapping_add(1);
        if code.is_empty() {
            s.formats.remove(&addr);
        } else {
            s.formats.insert(addr, code.to_string());
        }
        self.log_change(sheet, addr);
    }

    /// Clear a cell's display format (it falls back to `General`).
    pub fn clear_format(&mut self, sheet: SheetId, addr: CellAddress) {
        self.set_format(sheet, addr, "");
    }

    /// A cell's display format code, or `None` if it uses the default (`General`).
    pub fn get_format(&self, sheet: SheetId, addr: CellAddress) -> Option<&str> {
        self.sheets
            .get(sheet.0 as usize)?
            .formats
            .get(&addr)
            .map(String::as_str)
    }

    /// The cell's computed value as the **display string** it should show —
    /// its value run through its format code (or `General`). Numbers are
    /// formatted per the code (grouping, decimals, percent, dates…); text,
    /// booleans, and errors render naturally; an empty cell is `""`. This is the
    /// one call a renderer needs per visible cell.
    pub fn get_display(&self, sheet: SheetId, addr: CellAddress) -> String {
        let value = self.get_value(sheet, addr).unwrap_or(CellValue::Empty);
        display_value(&value, self.get_format(sheet, addr))
    }

    /// Recalculate every formula cell. Bumps the epoch on success.
    pub fn recalc_all(&mut self) {
        // A full sweep is one revision-transaction too, so the cells it rewrites
        // land in the change log under a single new revision.
        self.revision = self.revision.wrapping_add(1);
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
    // Structural edits — insert / delete rows & columns
    // ----------------------------------------------------------------
    //
    // These relabel the grid: cells at or past the edit point slide over, and
    // every formula's references are rewritten (via [`FormulaAst::adjust`]) so it
    // keeps naming the same logical cells — a reference to a deleted line becomes
    // `#REF!`. The pure address/AST arithmetic lives in `edit.rs`; this layer
    // applies it to the live cell store, rebuilds the dependency graph (every
    // address moved, so the old edges are stale), and recalculates.
    //
    // v1 scope: single-sheet (the engine's formula references are sheet-local),
    // and the rebuild + recalc is a full sweep — correct and simple. Cross-sheet
    // reference adjustment and an incremental recalc are future optimisations.

    /// Insert `count` blank rows before 1-based row `at`; rows at/after `at`
    /// slide down. Formulas are rewritten and the sheet recalculated.
    pub fn insert_rows(&mut self, sheet: SheetId, at: u32, count: u32) {
        self.apply_structural_edit(sheet, StructuralEdit::InsertRows { at, count });
    }

    /// Delete `count` rows starting at 1-based row `at`; rows after slide up.
    /// Cells on deleted rows are removed; references to them become `#REF!`.
    pub fn delete_rows(&mut self, sheet: SheetId, at: u32, count: u32) {
        self.apply_structural_edit(sheet, StructuralEdit::DeleteRows { at, count });
    }

    /// Insert `count` blank columns before 1-based column `at`; columns at/after
    /// slide right. Formulas are rewritten and the sheet recalculated.
    pub fn insert_cols(&mut self, sheet: SheetId, at: u32, count: u32) {
        self.apply_structural_edit(sheet, StructuralEdit::InsertCols { at, count });
    }

    /// Delete `count` columns starting at 1-based column `at`; columns after
    /// slide left. Cells on deleted columns are removed; references → `#REF!`.
    pub fn delete_cols(&mut self, sheet: SheetId, at: u32, count: u32) {
        self.apply_structural_edit(sheet, StructuralEdit::DeleteCols { at, count });
    }

    /// Apply a [`StructuralEdit`] to one sheet: relocate every cell, rewrite each
    /// formula's references and echo text, drop cells on deleted lines, rebuild
    /// the dependency graph, and recalculate. A no-op if `sheet` is unknown.
    fn apply_structural_edit(&mut self, sheet: SheetId, edit: StructuralEdit) {
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return;
        };

        // 0. Refuse an insert that would push a non-empty cell off the u32 grid
        //    edge. There the per-coordinate shift saturates at `u32::MAX`, so two
        //    distinct cells would collapse onto the same relocated address and the
        //    second would silently overwrite the first in the map below — data
        //    loss. Excel likewise refuses to shift non-empty cells off the sheet.
        //    (Deletes only drop a band and shift survivors inward — never a
        //    collision — so they need no such guard.)
        // Check both the cell store and the format store: a format can sit on an
        // empty (content-less) cell, so a format-only entry could collide too.
        let would_overflow_grid = match edit {
            StructuralEdit::InsertRows { at, count } => s
                .cells
                .keys()
                .chain(s.formats.keys())
                .any(|a| a.row >= at && a.row.checked_add(count).is_none()),
            StructuralEdit::InsertCols { at, count } => s
                .cells
                .keys()
                .chain(s.formats.keys())
                .any(|a| a.col >= at && a.col.checked_add(count).is_none()),
            StructuralEdit::DeleteRows { .. } | StructuralEdit::DeleteCols { .. } => false,
        };
        if would_overflow_grid {
            return; // reject the whole edit rather than lose cells
        }

        // 1. Relocate cells + rewrite formula references. A cell's *position*
        //    and its formula's *references* both follow the same edit. Build a
        //    fresh map: a cell on a deleted line has no new address → dropped.
        let old = std::mem::take(&mut s.cells);
        let mut moved: HashMap<CellAddress, Cell> = HashMap::with_capacity(old.len());
        for (addr, mut cell) in old {
            if let CellContent::Formula { ast, text, cached } = &mut cell.content {
                *ast = ast.adjust(edit);
                *text = ast.to_formula_string(); // keep the echo text honest
                *cached = None; // recomputed below
            }
            if let Some(new_addr) = addr.adjust(edit) {
                moved.insert(new_addr, cell);
            }
        }
        self.sheets[sheet.0 as usize].cells = moved;

        // 1b. Relocate the format store the same way (formats ride with the cell
        //     they decorate; a format on a deleted line is dropped).
        let old_formats = std::mem::take(&mut self.sheets[sheet.0 as usize].formats);
        let mut moved_formats: HashMap<CellAddress, String> =
            HashMap::with_capacity(old_formats.len());
        for (addr, code) in old_formats {
            if let Some(new_addr) = addr.adjust(edit) {
                moved_formats.insert(new_addr, code);
            }
        }
        self.sheets[sheet.0 as usize].formats = moved_formats;

        // 2. Every address moved, so the old dependency edges are stale. Rebuild
        //    the graph from the rewritten ASTs, then recalc the whole workbook.
        //    `recalc_all` bumps the revision (one transaction for the edit).
        self.rebuild_dependency_graph();
        self.recalc_all();

        // 3. `recalc_all` logged the formula cells it recomputed; also log the
        //    surviving literal cells so a viewport `changed_since` snapshot taken
        //    before the edit sees the relocation. (Deleted cells can't be logged
        //    — a host re-fetches its window, where they read back as empty.)
        let addrs: Vec<CellAddress> = self.sheets[sheet.0 as usize]
            .cells
            .keys()
            .copied()
            .collect();
        for a in addrs {
            self.log_change(sheet, a);
        }
    }

    /// Rebuild the entire cross-sheet dependency graph from the current formula
    /// ASTs. Used after a structural edit relocates addresses en masse, which
    /// invalidates every existing edge.
    fn rebuild_dependency_graph(&mut self) {
        // Collect first (can't borrow `self.sheets` and mutate `self.graph` at
        // once), then repopulate a fresh graph.
        // A graph node is a (sheet, address) pair; an entry is a node plus its
        // dependency nodes.
        type Node = (SheetId, CellAddress);
        let mut deps: Vec<(Node, Vec<Node>)> = Vec::new();
        for (i, s) in self.sheets.iter().enumerate() {
            let sheet = SheetId(i as u32);
            for (addr, cell) in &s.cells {
                if let CellContent::Formula { ast, .. } = &cell.content {
                    let mut refs = Vec::new();
                    collect_refs(ast, sheet, &mut refs);
                    deps.push(((sheet, *addr), refs));
                }
            }
        }
        self.graph = DependencyGraph::new();
        for (node, refs) in deps {
            self.graph.set_dependencies(node, refs);
        }
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
        {
            let s = &mut self.sheets[sheet.0 as usize];
            if let Some(cell) = s.cells.get_mut(&addr) {
                if let CellContent::Formula { cached, .. } = &mut cell.content {
                    *cached = Some(value);
                }
            }
        }
        // Every recompute is a (potential) value change for viewport diffing.
        // v1 stamps on write rather than diffing old vs new: a no-op recompute
        // logs the cell even if its value is unchanged. That over-reports at
        // worst — a host re-fetches a handful of identical cells, which is safe;
        // exact old/new diffing is a future optimisation.
        self.log_change(sheet, addr);
    }

    /// Append a change-log entry under the current revision, pruning the log to
    /// `CHANGELOG_RETAIN` and tracking the highest dropped revision.
    fn log_change(&mut self, sheet: SheetId, addr: CellAddress) {
        self.changes.push_back((self.revision, sheet, addr));
        while self.changes.len() > CHANGELOG_RETAIN {
            if let Some((dropped, _, _)) = self.changes.pop_front() {
                self.dropped_revision = self.dropped_revision.max(dropped);
            }
        }
    }
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a computed value as the display string a cell should show, under an
/// optional format code. Numbers go through `number-format-core` (the code, or
/// `General` when there's none); text / booleans / errors render naturally; an
/// empty cell is the empty string. Number formats apply only to numbers — a
/// format on a text or boolean cell is ignored, as in a spreadsheet.
fn display_value(value: &CellValue, format: Option<&str>) -> String {
    match value {
        CellValue::Number(n) => number_format_core::format_number(*n, format.unwrap_or("General")),
        CellValue::Empty => String::new(),
        CellValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::Text(s) => s.clone(),
        CellValue::Error(e) => e.display().to_string(),
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

    // ── Structural edits: insert / delete rows & columns ────────────

    #[test]
    fn insert_rows_relocates_cells_and_rewrites_formulas() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0)); // A1
        wb.set_value(s, cell(2, 1), CellValue::Number(20.0)); // A2
        wb.set_formula(s, cell(3, 1), "=SUM(A1:A2)").unwrap(); // A3 = 30
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(30.0)));

        // Insert one row at the top: everything slides down a row.
        wb.insert_rows(s, 1, 1);
        assert_eq!(wb.get_value(s, cell(1, 1)), None); // A1 now blank
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(10.0))); // was A1
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(20.0))); // was A2
        // The SUM moved to A4 and its range was rewritten A1:A2 → A2:A3; still 30.
        assert_eq!(wb.get_value(s, cell(4, 1)), Some(CellValue::Number(30.0)));
        // And editing a now-relocated input still ripples through.
        wb.set_value(s, cell(2, 1), CellValue::Number(110.0));
        assert_eq!(wb.get_value(s, cell(4, 1)), Some(CellValue::Number(130.0)));
    }

    #[test]
    fn delete_rows_shifts_survivors_and_rewrites_refs() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0)); // A1
        wb.set_value(s, cell(2, 1), CellValue::Number(20.0)); // A2
        wb.set_formula(s, cell(3, 1), "=A2*2").unwrap(); // A3 = 40

        wb.delete_rows(s, 1, 1); // delete row 1
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(20.0))); // was A2
        // The formula moved A3 → A2 and its ref A2 → A1; 20*2 = 40.
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(40.0)));
    }

    #[test]
    fn deleting_a_referenced_line_yields_ref_error() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0)); // A1
        wb.set_formula(s, cell(1, 2), "=A1+1").unwrap(); // B1 = 11

        wb.delete_cols(s, 1, 1); // delete column A — A1 is gone
        // B1 → A1, and its reference A1 (now deleted) → #REF!.
        assert_eq!(
            wb.get_value(s, cell(1, 1)),
            Some(CellValue::Error(SpreadsheetError::Ref))
        );
    }

    #[test]
    fn insert_cols_shifts_columns_right() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(5.0)); // A1
        wb.set_formula(s, cell(1, 2), "=A1*3").unwrap(); // B1 = 15

        wb.insert_cols(s, 1, 2); // two blank columns at the left
        assert_eq!(wb.get_value(s, cell(1, 1)), None); // A1 blank
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(5.0))); // was A1
        // B1 → D1, ref A1 → C1; still 15.
        assert_eq!(wb.get_value(s, cell(1, 4)), Some(CellValue::Number(15.0)));
    }

    #[test]
    fn structural_edit_advances_revision() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        let before = wb.current_revision();
        wb.insert_rows(s, 1, 1);
        assert!(wb.current_revision() > before);
    }

    // ── Cell formats / display ──────────────────────────────────────

    #[test]
    fn get_display_applies_format_or_defaults_to_general() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1234.5));
        // No format → General (shortest representation).
        assert_eq!(wb.get_display(s, cell(1, 1)), "1234.5");
        // With a format code.
        wb.set_format(s, cell(1, 1), "#,##0.00");
        assert_eq!(wb.get_display(s, cell(1, 1)), "1,234.50");
        assert_eq!(wb.get_format(s, cell(1, 1)), Some("#,##0.00"));
        // A percent format on a fraction.
        wb.set_value(s, cell(2, 1), CellValue::Number(0.5));
        wb.set_format(s, cell(2, 1), "0%");
        assert_eq!(wb.get_display(s, cell(2, 1)), "50%");
    }

    #[test]
    fn clear_format_reverts_to_general() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(0.5));
        wb.set_format(s, cell(1, 1), "0%");
        assert_eq!(wb.get_display(s, cell(1, 1)), "50%");
        wb.clear_format(s, cell(1, 1));
        assert_eq!(wb.get_format(s, cell(1, 1)), None);
        assert_eq!(wb.get_display(s, cell(1, 1)), "0.5"); // General
    }

    #[test]
    fn format_ignored_on_non_numbers() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Text("hi".into()));
        wb.set_format(s, cell(1, 1), "#,##0.00"); // numeric format on text
        assert_eq!(wb.get_display(s, cell(1, 1)), "hi"); // text renders naturally
        wb.set_formula(s, cell(1, 2), "=1/0").unwrap();
        wb.set_format(s, cell(1, 2), "0.00");
        assert_eq!(wb.get_display(s, cell(1, 2)), "#DIV/0!"); // error shows through
    }

    #[test]
    fn formatted_cells_compute_display_through_a_formula() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1500.0));
        wb.set_value(s, cell(2, 1), CellValue::Number(2500.0));
        wb.set_formula(s, cell(3, 1), "=A1+A2").unwrap(); // 4000
        wb.set_format(s, cell(3, 1), "#,##0");
        assert_eq!(wb.get_display(s, cell(3, 1)), "4,000");
    }

    #[test]
    fn format_relocates_with_its_cell_on_insert_and_drops_on_delete() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1234.5));
        wb.set_format(s, cell(1, 1), "#,##0.00");

        wb.insert_rows(s, 1, 1); // A1 → A2; the format must ride along
        assert_eq!(wb.get_format(s, cell(2, 1)), Some("#,##0.00"));
        assert_eq!(wb.get_display(s, cell(2, 1)), "1,234.50");
        assert_eq!(wb.get_format(s, cell(1, 1)), None); // nothing at A1 now

        wb.delete_rows(s, 2, 1); // delete the row holding the formatted cell
        assert_eq!(wb.get_format(s, cell(1, 1)), None); // format dropped with the cell
    }

    #[test]
    fn insert_that_would_overflow_the_grid_is_rejected_not_lossy() {
        // An insert whose shift saturates u32 would collapse distinct cells onto
        // u32::MAX in the relocation map, silently dropping one. The guard must
        // reject the whole edit so no cell is lost.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0)); // A1
        wb.set_value(s, cell(2, 1), CellValue::Number(20.0)); // A2
        wb.insert_rows(s, 1, u32::MAX); // both rows would saturate to u32::MAX
        // No-op: both cells survive, unmoved.
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(10.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(20.0)));
    }

    #[test]
    fn structural_edit_on_unknown_sheet_is_a_noop() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.insert_rows(SheetId(99), 1, 1); // no such sheet
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
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

    // ── Viewport primitive (infinite virtualized sheet) ──────────────

    #[test]
    fn get_window_is_dense_with_blanks() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(1, 3), CellValue::Number(3.0)); // B1 (col 2) left empty
        let w = wb.get_window(s, 1, 1, 1, 3).unwrap();
        assert_eq!((w.rows, w.cols), (1, 3));
        assert_eq!(w.values, vec![
            CellValue::Number(1.0), // A1
            CellValue::Empty,       // B1 — blank, not omitted
            CellValue::Number(3.0), // C1
        ]);
        assert_eq!(w.get(1, 2), Some(&CellValue::Empty));
        assert_eq!(w.get(1, 3), Some(&CellValue::Number(3.0)));
    }

    #[test]
    fn get_window_returns_computed_formula_values() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(15.0));
        wb.set_value(s, cell(1, 2), CellValue::Number(3.0));
        wb.set_formula(s, cell(1, 3), "=SUM(A1:B1)").unwrap();
        let w = wb.get_window(s, 1, 3, 1, 3).unwrap();
        assert_eq!(w.values, vec![CellValue::Number(18.0)]);
    }

    #[test]
    fn get_window_far_flung_cell_is_cheap() {
        // A cell at row 1,000,000, col 1000 is reachable without materialising
        // the rectangle between it and the origin — the read is O(window).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1_000_000, 1_000), CellValue::Number(42.0));
        let w = wb.get_window(s, 1_000_000, 1_000, 1_000_000, 1_000).unwrap();
        assert_eq!(w.values, vec![CellValue::Number(42.0)]);
    }

    #[test]
    fn get_window_rejects_inverted_and_oversized() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        assert!(wb.get_window(s, 2, 1, 1, 1).is_err()); // row1 < row0
        assert!(wb.get_window(s, 1, 2, 1, 1).is_err()); // col1 < col0
        // 400×400 = 160 000 > MAX_WINDOW_CELLS (65 536).
        assert!(wb.get_window(s, 1, 1, 400, 400).is_err());
        // Full-sheet request: rows*cols overflows u64 → still rejected, no panic.
        assert!(wb.get_window(s, 1, 1, u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn get_window_full_u32_span_is_rejected_not_a_dos() {
        // Regression: the span must be computed in u64. A full-u32-span request
        // (row1 - row0 == u32::MAX) would overflow the u32 `+ 1`, wrap to a bogus
        // small count, slip past the cap, and loop over the entire u32 range
        // (OOM in release, panic in debug). row0 = 0 is also out of the 1-based
        // contract. All of these must return Err WITHOUT panicking or hanging.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        assert!(wb.get_window(s, 0, 0, u32::MAX, u32::MAX).is_err()); // row0=0 + full span
        assert!(wb.get_window(s, 0, 1, 10, 10).is_err()); // 0 violates 1-based
        assert!(wb.get_window(s, 1, 0, 10, 10).is_err());
        assert!(wb.get_window(s, 1, 1, u32::MAX, 1).is_err()); // tall full-span column
        assert!(wb.get_window(s, 1, 1, 1, u32::MAX).is_err()); // wide full-span row
    }

    #[test]
    fn used_range_none_when_empty_some_when_scattered() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        assert_eq!(wb.used_range(s), None);
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0)); // A1
        wb.set_value(s, cell(100, 26), CellValue::Number(2.0)); // Z100
        assert_eq!(wb.used_range(s), Some(UsedRange {
            min_row: 1, min_col: 1, max_row: 100, max_col: 26,
        }));
    }

    #[test]
    fn used_range_skips_present_but_empty_cells() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(9, 9), CellValue::Empty); // present but empty
        // The Empty cell must not extend the extent.
        assert_eq!(wb.used_range(s), Some(UsedRange {
            min_row: 1, min_col: 1, max_row: 1, max_col: 1,
        }));
    }

    #[test]
    fn current_revision_advances_per_edit() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        let r0 = wb.current_revision();
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        let r1 = wb.current_revision();
        wb.set_value(s, cell(1, 2), CellValue::Number(2.0));
        let r2 = wb.current_revision();
        assert!(r1 > r0 && r2 > r1);
    }

    #[test]
    fn changed_since_reports_edited_cell_and_its_dependents() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_formula(s, cell(1, 2), "=A1*10").unwrap();
        let snap = wb.current_revision();
        wb.set_value(s, cell(1, 1), CellValue::Number(9.0)); // A1 + dependent B1
        match wb.changed_since(s, snap) {
            ChangeSet::Delta { changed, .. } => {
                assert!(changed.contains(&cell(1, 1)), "A1 changed");
                assert!(changed.contains(&cell(1, 2)), "B1 (dependent) changed");
                assert_eq!(changed.len(), 2, "deduped");
            }
            ChangeSet::Stale { .. } => panic!("should be a Delta"),
        }
    }

    #[test]
    fn changed_since_filters_by_sheet() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("S1");
        let s2 = wb.add_sheet("S2");
        let snap = wb.current_revision();
        wb.set_value(s1, cell(1, 1), CellValue::Number(1.0));
        // The edit on S1 must not appear when querying S2.
        match wb.changed_since(s2, snap) {
            ChangeSet::Delta { changed, .. } => assert!(changed.is_empty()),
            ChangeSet::Stale { .. } => panic!("should be a Delta"),
        }
    }

    #[test]
    fn changed_since_goes_stale_past_the_retained_window() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        let snap = wb.current_revision(); // 0
        // Edit far more distinct cells than the log retains, forcing it to drop
        // the oldest entries — a query reaching back to `snap` can't be proven
        // complete, so it must answer Stale (re-read everything).
        for i in 1..=(CHANGELOG_RETAIN as u32 + 100) {
            wb.set_value(s, cell(i, 1), CellValue::Number(i as f64));
        }
        assert!(matches!(
            wb.changed_since(s, snap),
            ChangeSet::Stale { .. }
        ));
        // But a recent snapshot still returns a Delta.
        let recent = wb.current_revision();
        wb.set_value(s, cell(1, 1), CellValue::Number(0.0));
        assert!(matches!(
            wb.changed_since(s, recent),
            ChangeSet::Delta { .. }
        ));
    }
}
