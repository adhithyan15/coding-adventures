//! The Workbook — the top-level container.
//!
//! Holds sheets, cells, the dependency graph, and the recalc epoch.
//! Phase 1 ships a minimal but complete engine: literal + formula
//! cells, dependency tracking, automatic-recalc-on-edit (the user
//! can also call `recalc_all` for a full sweep).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::address::{CellAddress, CellRange, SheetId, MAX_RANGE_CELLS};
use crate::cell::{Cell, CellContent, CellValue};
use crate::dag::DependencyGraph;
use crate::edit::{delete_coord, insert_coord, StructuralEdit};
use crate::errors::SpreadsheetError;
use crate::parser::{parse, ParseError};
use crate::recalc::{collect_refs, evaluate};
use crate::viewport::{ChangeSet, DisplayWindow, UsedRange, Window, CHANGELOG_RETAIN, MAX_WINDOW_CELLS};

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
    /// The clipboard buffer set by [`copy`]/[`cut`] and consumed by [`paste`].
    /// `None` until something is copied. A copy survives any number of pastes; a
    /// cut is one-shot (the buffer is taken on the paste that moves it).
    ///
    /// [`copy`]: Workbook::copy
    /// [`cut`]: Workbook::cut
    /// [`paste`]: Workbook::paste
    clipboard: Option<Clipboard>,
}

/// A copied/cut rectangle, captured relative to the source range's top-left
/// anchor so it can be pasted anywhere. Content + format ride along; on paste
/// the whole block shifts its references by `dst_anchor − anchor`.
struct Clipboard {
    /// The sheet the snapshot came from.
    sheet: SheetId,
    /// The source range's top-left cell — paste shifts references by the
    /// destination anchor's offset from here.
    anchor: CellAddress,
    /// The full source rectangle (kept so a `cut` paste can clear the origin).
    source: CellRange,
    /// Source rectangle size; paste clears this whole rectangle at the
    /// destination, so blank source cells erase their targets (Excel-style).
    rows: u32,
    cols: u32,
    /// The non-blank source cells, each as a `(d_row, d_col)` offset from
    /// `anchor` plus its content and format. Sparse: blank cells are omitted and
    /// reconstructed as "clear the target" from `rows`×`cols`.
    cells: Vec<ClipCell>,
    /// `true` for a cut — paste clears the source cells it didn't overwrite and
    /// then consumes the buffer. `false` for a copy (repeatable paste).
    is_cut: bool,
}

/// One snapshotted cell in a [`Clipboard`], at `(d_row, d_col)` from the anchor.
struct ClipCell {
    d_row: u32,
    d_col: u32,
    content: CellContent,
    format: Option<String>,
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
    /// Per-column widths and per-row heights, keyed by the 1-based column / row
    /// index. The value is an **opaque `f64` in host units** (the engine never
    /// computes with it — it only stores, key-shifts on structural edits, and
    /// serializes it). A column / row *absent* from the map uses the host's
    /// default size, so a fresh sheet (both maps empty) is byte-identical to the
    /// pre-feature behaviour. See [`set_column_width`] / [`set_row_height`].
    ///
    /// [`set_column_width`]: Workbook::set_column_width
    /// [`set_row_height`]: Workbook::set_row_height
    col_widths: HashMap<u32, f64>,
    row_heights: HashMap<u32, f64>,
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
            clipboard: None,
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
            col_widths: HashMap::new(),
            row_heights: HashMap::new(),
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

    /// All sheet names in tab order (`SheetId(i)` is the i-th name). Drives a
    /// host's sheet tab bar.
    pub fn sheet_names(&self) -> Vec<&str> {
        self.sheets.iter().map(|s| s.name.as_str()).collect()
    }

    /// Rebuild the `name → SheetId` index from the current sheet order. Called
    /// after any operation that changes names or reorders/removes sheets (so the
    /// dense `SheetId` indices stay in sync with the `Vec`).
    fn rebuild_sheet_index(&mut self) {
        self.sheet_by_name.clear();
        for (i, s) in self.sheets.iter().enumerate() {
            self.sheet_by_name.insert(s.name.clone(), SheetId(i as u32));
        }
    }

    /// Rename a sheet. The sheet's `SheetId` is unchanged (so the dependency graph
    /// and computed values are untouched — a rename is purely cosmetic), but every
    /// formula that *names* the old sheet has its qualifier rewritten to the new
    /// name in its stored source (`=Old!A1` → `=New!A1`). Rejects an empty name or
    /// one already used by a different sheet; renaming to the current name is a
    /// no-op. Bumps the revision so a host re-reads the affected sources.
    pub fn rename_sheet(
        &mut self,
        sheet: SheetId,
        new_name: impl Into<String>,
    ) -> Result<(), String> {
        let new_name = new_name.into();
        let idx = sheet.0 as usize;
        if idx >= self.sheets.len() {
            return Err("unknown sheet".to_string());
        }
        if new_name.is_empty() {
            return Err("sheet name must not be empty".to_string());
        }
        let old_name = self.sheets[idx].name.clone();
        if old_name == new_name {
            return Ok(());
        }
        if let Some(existing) = self.sheet_by_name.get(&new_name) {
            if *existing != sheet {
                return Err(format!("a sheet named {new_name:?} already exists"));
            }
        }
        self.sheets[idx].name = new_name.clone();
        self.rebuild_sheet_index();
        // Rewrite the qualifier in every formula that named the old sheet (any
        // sheet may reference it). Values are unchanged, so no recalc is needed.
        for s in &mut self.sheets {
            for cell in s.cells.values_mut() {
                if let CellContent::Formula { ast, text, .. } = &mut cell.content {
                    let renamed = ast.rename_qualifier(&old_name, &new_name);
                    if renamed != *ast {
                        *ast = renamed;
                        *text = ast.to_formula_string();
                    }
                }
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Delete a sheet. Refuses to remove the last sheet (a workbook always has at
    /// least one). Removing the sheet shifts every later sheet's dense `SheetId`
    /// down by one, so the name index and dependency graph are rebuilt afterwards.
    /// Every reference that pointed *into* the deleted sheet is rewritten to the
    /// `#REF!` error literal (permanent — re-adding a same-named sheet doesn't
    /// resurrect it), then the workbook recomputes.
    pub fn delete_sheet(&mut self, sheet: SheetId) -> Result<(), String> {
        let idx = sheet.0 as usize;
        if idx >= self.sheets.len() {
            return Err("unknown sheet".to_string());
        }
        if self.sheets.len() <= 1 {
            return Err("cannot delete the last sheet".to_string());
        }
        let removed_name = self.sheets[idx].name.clone();
        self.sheets.remove(idx);
        self.rebuild_sheet_index();
        // Inbound refs to the now-gone sheet collapse to #REF!.
        for s in &mut self.sheets {
            for cell in s.cells.values_mut() {
                if let CellContent::Formula { ast, text, cached } = &mut cell.content {
                    let rewritten = ast.sheet_refs_to_error(&removed_name);
                    if rewritten != *ast {
                        *ast = rewritten;
                        *text = ast.to_formula_string();
                        *cached = None;
                    }
                }
            }
        }
        self.rebuild_dependency_graph();
        self.recalc_all(); // bumps revision + epoch
        Ok(())
    }

    /// Reorder a sheet to a new 0-based tab position (clamped into range). The
    /// move changes dense `SheetId`s, so the name index and dependency graph are
    /// rebuilt; names — and therefore every formula's qualifiers and computed
    /// values — are unaffected. A move to the current position is a no-op.
    pub fn move_sheet(&mut self, sheet: SheetId, to_index: usize) -> Result<(), String> {
        let idx = sheet.0 as usize;
        if idx >= self.sheets.len() {
            return Err("unknown sheet".to_string());
        }
        let to = to_index.min(self.sheets.len() - 1);
        if to == idx {
            return Ok(());
        }
        let s = self.sheets.remove(idx);
        self.sheets.insert(to, s);
        self.rebuild_sheet_index();
        self.rebuild_dependency_graph();
        self.recalc_all(); // re-resolve names → new ids; values unchanged
        Ok(())
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
        let (rows, cols) = window_dims(row0, col0, row1, col1)?;
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

    /// Like [`get_window`](Self::get_window), but each cell is the **display
    /// string** it should paint — its value rendered through its format code
    /// (see [`get_display`](Self::get_display)) — rather than a typed value. This
    /// is the one read a virtualized grid needs per frame: a dense rectangle of
    /// ready-to-draw, format-applied strings. Same 1-based coords and
    /// [`MAX_WINDOW_CELLS`](crate::viewport::MAX_WINDOW_CELLS) cap as `get_window`.
    pub fn get_display_window(
        &self,
        sheet: SheetId,
        row0: u32,
        col0: u32,
        row1: u32,
        col1: u32,
    ) -> Result<DisplayWindow, SpreadsheetError> {
        let (rows, cols) = window_dims(row0, col0, row1, col1)?;
        // Validate the sheet exists up front (get_display would silently yield
        // "" for an unknown sheet, but the windowed read promises a Ref error).
        if self.sheets.get(sheet.0 as usize).is_none() {
            return Err(SpreadsheetError::Ref);
        }
        let mut cells = Vec::with_capacity((rows * cols) as usize);
        for r in row0..=row1 {
            for c in col0..=col1 {
                cells.push(self.get_display(sheet, CellAddress::new(r, c)));
            }
        }
        Ok(DisplayWindow {
            row0,
            col0,
            rows: rows as u32,
            cols: cols as u32,
            cells,
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
        // Update dependency edges. A cross-sheet ref registers an edge into its
        // target sheet (resolved via the sheet-name map), so editing that sheet's
        // cell recomputes this formula through the cross-sheet graph.
        let mut refs = Vec::new();
        {
            let resolve = |name: &str| self.sheet_by_name.get(name).copied();
            collect_refs(&ast, sheet, &resolve, &mut refs);
        }
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

    // ── Column widths & row heights ──────────────────────────────────────
    // Per-sheet presentation chrome: the engine STORES a width/height keyed by
    // column/row index but never reads it for any computation. A host renders
    // columns/rows at these sizes; an index with no stored size uses the host's
    // own default. The value is an opaque `f64` in whatever unit the host picks
    // (the demos use pixels). These survive save/load and shift with their
    // column/row on a structural edit (see `apply_structural_edit`).

    /// The stored width of a 1-based `col` on `sheet`, or `None` if the column has
    /// no custom width (the host should use its default). `None` for an unknown
    /// sheet or `col == 0`.
    pub fn column_width(&self, sheet: SheetId, col: u32) -> Option<f64> {
        self.sheets
            .get(sheet.0 as usize)
            .and_then(|s| s.col_widths.get(&col).copied())
    }

    /// The stored height of a 1-based `row` on `sheet`, or `None` if the row has
    /// no custom height. `None` for an unknown sheet or `row == 0`.
    pub fn row_height(&self, sheet: SheetId, row: u32) -> Option<f64> {
        self.sheets
            .get(sheet.0 as usize)
            .and_then(|s| s.row_heights.get(&row).copied())
    }

    /// Every customized column width in the inclusive 1-based range `[col0, col1]`
    /// on `sheet`, as `(col, width)` pairs sorted by column. Columns with no custom
    /// width are omitted — a host fetches a viewport's overrides in one call instead
    /// of one probe per column. Empty for an unknown sheet or an empty range.
    pub fn column_widths_in(&self, sheet: SheetId, col0: u32, col1: u32) -> Vec<(u32, f64)> {
        let Some(s) = self.sheets.get(sheet.0 as usize) else {
            return Vec::new();
        };
        let mut out: Vec<(u32, f64)> = s
            .col_widths
            .iter()
            .filter(|(c, _)| **c >= col0 && **c <= col1)
            .map(|(c, w)| (*c, *w))
            .collect();
        out.sort_by_key(|(c, _)| *c);
        out
    }

    /// Every customized row height in the inclusive 1-based range `[row0, row1]` on
    /// `sheet`, as `(row, height)` pairs sorted by row. The row analogue of
    /// [`column_widths_in`].
    ///
    /// [`column_widths_in`]: Workbook::column_widths_in
    pub fn row_heights_in(&self, sheet: SheetId, row0: u32, row1: u32) -> Vec<(u32, f64)> {
        let Some(s) = self.sheets.get(sheet.0 as usize) else {
            return Vec::new();
        };
        let mut out: Vec<(u32, f64)> = s
            .row_heights
            .iter()
            .filter(|(r, _)| **r >= row0 && **r <= row1)
            .map(|(r, h)| (*r, *h))
            .collect();
        out.sort_by_key(|(r, _)| *r);
        out
    }

    /// Set the width of a 1-based `col` on `sheet`. Returns `true` if the map
    /// changed. Rejects (returns `false`, leaving the map untouched) a non-finite
    /// width (`NaN` / `±∞`), a width `≤ 0`, `col == 0`, or an unknown sheet — so a
    /// bad host value can never poison the map or the serialized file. Setting the
    /// width a column already has is a no-op (no revision bump), matching the
    /// engine's diff-gating convention so an unchanged resize isn't snapshotted.
    pub fn set_column_width(&mut self, sheet: SheetId, col: u32, width: f64) -> bool {
        if col == 0 || !width.is_finite() || width <= 0.0 {
            return false;
        }
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return false;
        };
        if s.col_widths.get(&col) == Some(&width) {
            return false; // unchanged
        }
        s.col_widths.insert(col, width);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Set the height of a 1-based `row` on `sheet`. The row analogue of
    /// [`set_column_width`] — same finite / `> 0` / `row ≥ 1` validation and
    /// same-value no-op.
    ///
    /// [`set_column_width`]: Workbook::set_column_width
    pub fn set_row_height(&mut self, sheet: SheetId, row: u32, height: f64) -> bool {
        if row == 0 || !height.is_finite() || height <= 0.0 {
            return false;
        }
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return false;
        };
        if s.row_heights.get(&row) == Some(&height) {
            return false; // unchanged
        }
        s.row_heights.insert(row, height);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Remove a column's custom width, returning it to the host default. Returns
    /// `true` if an entry was removed (and bumps the revision); `false` if the
    /// column had no custom width or the sheet is unknown.
    pub fn clear_column_width(&mut self, sheet: SheetId, col: u32) -> bool {
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return false;
        };
        if s.col_widths.remove(&col).is_some() {
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Remove a row's custom height, returning it to the host default. The row
    /// analogue of [`clear_column_width`].
    ///
    /// [`clear_column_width`]: Workbook::clear_column_width
    pub fn clear_row_height(&mut self, sheet: SheetId, row: u32) -> bool {
        let Some(s) = self.sheets.get_mut(sheet.0 as usize) else {
            return false;
        };
        if s.row_heights.remove(&row).is_some() {
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
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

        // The name of the edited sheet — so an inbound `S!…` reference on another
        // sheet can be recognised and shifted (see step 1c). Always `Some` for a
        // valid sheet; default to "" (no real sheet is named "") if somehow absent.
        let edited_name = s.name.clone();

        // 1. Relocate the EDITED sheet's cells + rewrite each formula's references.
        //    A cell's *position* and its formula's *references into this sheet*
        //    both follow the edit. `adjust_for_sheet_edit(.., edited_is_host=true,
        //    ..)` shifts this sheet's unqualified refs (and any self-qualified
        //    `S!…` refs), while leaving refs into *other* sheets untouched. Build a
        //    fresh map: a cell on a deleted line has no new address → dropped.
        let old = std::mem::take(&mut s.cells);
        let mut moved: HashMap<CellAddress, Cell> = HashMap::with_capacity(old.len());
        for (addr, mut cell) in old {
            if let CellContent::Formula { ast, text, cached } = &mut cell.content {
                *ast = ast.adjust_for_sheet_edit(edit, true, &edited_name);
                *text = ast.to_formula_string(); // keep the echo text honest
                *cached = None; // recomputed below
            }
            if let Some(new_addr) = addr.adjust(edit) {
                moved.insert(new_addr, cell);
            }
        }
        self.sheets[sheet.0 as usize].cells = moved;

        // 1c. Inbound cross-sheet propagation: every OTHER sheet's formulas may hold
        //     a `S!…` reference into the edited sheet, which must shift too (the
        //     grid those refs name just moved). Walk each non-edited sheet and
        //     adjust only its refs qualified with `edited_name`
        //     (`edited_is_host = false`), leaving its own-sheet and other-sheet refs
        //     alone. Cells are NOT relocated here — only the *edited* sheet's cells
        //     move; these are just reference rewrites on cells that stay put.
        for other in 0..self.sheets.len() {
            if other == sheet.0 as usize {
                continue;
            }
            let cells = std::mem::take(&mut self.sheets[other].cells);
            let rewritten = cells
                .into_iter()
                .map(|(addr, mut cell)| {
                    if let CellContent::Formula { ast, text, cached } = &mut cell.content {
                        let new_ast = ast.adjust_for_sheet_edit(edit, false, &edited_name);
                        if new_ast != *ast {
                            *ast = new_ast;
                            *text = ast.to_formula_string();
                            *cached = None;
                        }
                    }
                    (addr, cell)
                })
                .collect();
            self.sheets[other].cells = rewritten;
        }

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

        // 1d. Shift the column-width / row-height keys the same way. A width is
        //     keyed by COLUMN index, so only a column insert/delete moves it; a
        //     height by ROW index, so only a row insert/delete. The OTHER axis is
        //     untouched (widen column C, insert a row → column C stays wide). A key
        //     in a deleted band is dropped (`delete_coord → None`), exactly as a
        //     cell/format on a deleted line is. Reuses the same `insert_coord` /
        //     `delete_coord` helpers that shift cell addresses and references, so
        //     the chrome stays aligned with the data it decorates.
        let s = &mut self.sheets[sheet.0 as usize];
        match edit {
            StructuralEdit::InsertCols { at, count } => {
                s.col_widths = shift_axis_keys(std::mem::take(&mut s.col_widths), |c| {
                    Some(insert_coord(c, at, count))
                });
            }
            StructuralEdit::DeleteCols { at, count } => {
                s.col_widths = shift_axis_keys(std::mem::take(&mut s.col_widths), |c| {
                    delete_coord(c, at, count)
                });
            }
            StructuralEdit::InsertRows { at, count } => {
                s.row_heights = shift_axis_keys(std::mem::take(&mut s.row_heights), |r| {
                    Some(insert_coord(r, at, count))
                });
            }
            StructuralEdit::DeleteRows { at, count } => {
                s.row_heights = shift_axis_keys(std::mem::take(&mut s.row_heights), |r| {
                    delete_coord(r, at, count)
                });
            }
        }

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

    // ----------------------------------------------------------------
    // Fill / replicate (drag-fill)
    // ----------------------------------------------------------------

    /// Replicate the cell at `src` across every cell of `dst` — the engine side
    /// of drag-fill / copy-paste.
    ///
    /// Each target gets a **copy** of the source, with its formula's references
    /// shifted by the target's offset from `src` (so `=A1` filled down the column
    /// becomes `=A2`, `=A3`, …), via [`FormulaAst::shift`] — relative refs track,
    /// absolute (`$`) refs stay pinned, and a ref shifted off the grid edge
    /// becomes `#REF!`. A literal source is copied unchanged; an **empty** source
    /// clears each target (filling "nothing" erases). The source's display
    /// **format** rides along to every target, the way a spreadsheet's fill does.
    /// `src` itself is overwritten with an identical copy when it falls inside
    /// `dst` (a zero offset — a no-op in effect).
    ///
    /// One recalc transaction (`recalc_all` bumps the revision once). Unknown
    /// `sheet` is a no-op. A `dst` larger than [`MAX_RANGE_CELLS`] is rejected
    /// wholesale (the same DoS guard formula ranges use) — a hostile or buggy
    /// caller can't ask the engine to materialise billions of cells.
    pub fn fill(&mut self, sheet: SheetId, src: CellAddress, dst: CellRange) {
        if self.sheets.get(sheet.0 as usize).is_none() {
            return;
        }
        // DoS guard: cap the number of cells a single fill can write. Computed in
        // u64 (cell_count already is), so it can't overflow on a full-grid range.
        if dst.cell_count() > MAX_RANGE_CELLS {
            return;
        }

        // Snapshot the source content + format up front: the loop overwrites
        // cells (possibly `src` itself), so we must not read it mid-fill.
        let s = &self.sheets[sheet.0 as usize];
        let src_content = s.cells.get(&src).map(|c| c.content.clone());
        let src_format = s.formats.get(&src).cloned();

        // Write every target's content + format directly (one transaction; the
        // single recalc_all below evaluates them all and bumps the revision once).
        for row in dst.start.row..=dst.end.row {
            for col in dst.start.col..=dst.end.col {
                let target = CellAddress::new(row, col);
                // Offsets in i64 then clamped into shift's i32 contract: row/col
                // are u32 (up to ~4.3e9), so `row as i32 - src.row as i32` would
                // overflow — a panic in debug/test builds, a silent wrap in
                // release — for a fill anchored at a high coordinate (which clears
                // the cell_count guard). i64 makes the subtraction exact for all
                // u32 operands; the clamp keeps it in range, and any resulting
                // off-grid reference still collapses to #REF! inside `shift`.
                let d_row =
                    (row as i64 - src.row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                let d_col =
                    (col as i64 - src.col as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;

                let new_content = match &src_content {
                    // Source never set / explicitly empty → clear the target.
                    None | Some(CellContent::Empty) => CellContent::Empty,
                    Some(CellContent::Value(v)) => CellContent::Value(v.clone()),
                    Some(CellContent::Formula { ast, .. }) => {
                        let shifted = ast.shift(d_row, d_col);
                        CellContent::Formula {
                            text: shifted.to_formula_string(),
                            ast: shifted,
                            cached: None, // recomputed by recalc_all
                        }
                    }
                };

                let s = &mut self.sheets[sheet.0 as usize];
                match new_content {
                    CellContent::Empty => {
                        s.cells.remove(&target);
                    }
                    content => {
                        s.cells.insert(target, Cell { content });
                    }
                }
                // The format rides with the cell: copy it, or clear the target's
                // format when the source had none.
                match &src_format {
                    Some(code) => {
                        s.formats.insert(target, code.clone());
                    }
                    None => {
                        s.formats.remove(&target);
                    }
                }
            }
        }

        // Targets' references changed en masse; rebuild edges, then recalc the
        // whole workbook (one revision bump). Log every target so a viewport
        // `changed_since` snapshot taken before the fill sees them.
        self.rebuild_dependency_graph();
        self.recalc_all();
        for row in dst.start.row..=dst.end.row {
            for col in dst.start.col..=dst.end.col {
                self.log_change(sheet, CellAddress::new(row, col));
            }
        }
    }

    // ----------------------------------------------------------------
    // Clipboard — cut / copy / paste
    // ----------------------------------------------------------------

    /// Copy `range` into the clipboard, capturing each cell's content **and**
    /// format relative to the range's top-left anchor. The source is left
    /// untouched and the buffer survives any number of [`paste`]s.
    ///
    /// This is `fill`'s sibling, generalised from one source cell to a whole
    /// rectangle: where fill shifts each target's references by its own offset
    /// from the source, a copy preserves the block's internal structure and
    /// shifts it as a unit on paste (`=A1` copied two columns right pastes as
    /// `=C1`). A `range` larger than [`MAX_RANGE_CELLS`] is rejected (the same
    /// DoS guard fill and formula ranges use); an unknown `sheet` is a no-op.
    ///
    /// [`paste`]: Workbook::paste
    pub fn copy(&mut self, sheet: SheetId, range: CellRange) {
        self.snapshot(sheet, range, false);
    }

    /// Cut `range` into the clipboard. Identical to [`copy`] except the buffer is
    /// marked as a move: the [`paste`] that places it clears the source cells it
    /// did not overwrite and then consumes the buffer (a cut pastes once).
    ///
    /// The source is *not* cleared here — only on paste — so a cut with no
    /// following paste leaves the sheet unchanged, matching a spreadsheet's
    /// "marching ants" behaviour.
    ///
    /// [`copy`]: Workbook::copy
    /// [`paste`]: Workbook::paste
    pub fn cut(&mut self, sheet: SheetId, range: CellRange) {
        self.snapshot(sheet, range, true);
    }

    /// Shared capture for [`copy`]/[`cut`]: snapshot the non-blank cells of
    /// `range` (content + format) as offsets from its anchor.
    fn snapshot(&mut self, sheet: SheetId, range: CellRange, is_cut: bool) {
        if self.sheets.get(sheet.0 as usize).is_none() {
            return;
        }
        // DoS guard: cap the rectangle a single copy/cut can capture.
        if range.cell_count() > MAX_RANGE_CELLS {
            return;
        }

        let anchor = range.start;
        let rows = range.end.row - range.start.row + 1;
        let cols = range.end.col - range.start.col + 1;

        let s = &self.sheets[sheet.0 as usize];
        let mut cells = Vec::new();
        for row in range.start.row..=range.end.row {
            for col in range.start.col..=range.end.col {
                let addr = CellAddress::new(row, col);
                let content = s.cells.get(&addr).map(|c| c.content.clone());
                let format = s.formats.get(&addr).cloned();
                // Skip cells that are blank in both content and format — they
                // erase their targets, which `paste` already does for any
                // offset not present in `cells`.
                if content.is_none() && format.is_none() {
                    continue;
                }
                cells.push(ClipCell {
                    d_row: row - anchor.row,
                    d_col: col - anchor.col,
                    content: content.unwrap_or(CellContent::Empty),
                    format,
                });
            }
        }

        self.clipboard = Some(Clipboard {
            sheet,
            anchor,
            source: range,
            rows,
            cols,
            cells,
            is_cut,
        });
    }

    /// Paste the clipboard so its top-left lands at `dst_anchor` on `sheet`.
    ///
    /// The whole block's references shift by `dst_anchor − anchor` via
    /// [`FormulaAst::shift`] (relative refs track, absolute `$` refs pin, a ref
    /// pushed off the grid becomes `#REF!`); content and format ride along. Every
    /// cell of the destination rectangle is written, so blanks in the source
    /// erase their targets. A **cut** then clears the source cells it didn't
    /// overwrite and consumes the buffer; a **copy** leaves the buffer for reuse.
    ///
    /// No-op (returning `false`) if the clipboard is empty, `sheet` is unknown,
    /// or the destination rectangle would run past the grid's last row/column —
    /// the engine never silently truncates or wraps an off-grid paste. Returns
    /// `true` when a paste was applied.
    ///
    /// > Note: for a cut, the moved formulas' own references are *shifted* like a
    /// > copy, rather than preserved as Excel's move does (Excel also rewrites
    /// > outside references that pointed into the moved range). This keeps cut a
    /// > thin layer over the copy machinery; the divergence is documented here
    /// > and in the spec.
    pub fn paste(&mut self, sheet: SheetId, dst_anchor: CellAddress) -> bool {
        // Take the buffer up front; restore it for a copy (repeatable), drop it
        // for a cut (one-shot). Avoids borrowing `self.clipboard` across the
        // mutations below.
        let clip = match self.clipboard.take() {
            Some(c) => c,
            None => return false,
        };
        if self.sheets.get(sheet.0 as usize).is_none() {
            self.clipboard = Some(clip); // sheet gone — leave the buffer intact
            return false;
        }

        // Reject a paste whose rectangle would extend past the u32 grid edge,
        // rather than wrap or clamp. `rows`/`cols` are ≥ 1, so subtract before
        // adding to avoid overflow.
        let last_row = dst_anchor.row as u64 + (clip.rows - 1) as u64;
        let last_col = dst_anchor.col as u64 + (clip.cols - 1) as u64;
        if last_row > u32::MAX as u64 || last_col > u32::MAX as u64 {
            self.clipboard = Some(clip);
            return false;
        }

        // The block shifts by the destination anchor's offset from the source
        // anchor. i64 then clamped into shift's i32 contract — u32 coordinates
        // up to ~4.3e9 would overflow an i32 subtraction (panic in debug, wrap in
        // release); the clamp keeps it in range and any off-grid ref still
        // collapses to #REF! inside `shift`.
        let d_row =
            (dst_anchor.row as i64 - clip.anchor.row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let d_col =
            (dst_anchor.col as i64 - clip.anchor.col as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        // Index the snapshot by offset for O(1) lookup as we sweep the rectangle.
        let mut by_offset: HashMap<(u32, u32), &ClipCell> = HashMap::new();
        for c in &clip.cells {
            by_offset.insert((c.d_row, c.d_col), c);
        }

        // Write every cell of the destination rectangle: snapshot cells get the
        // shifted content + format, blanks clear their target.
        for dr in 0..clip.rows {
            for dc in 0..clip.cols {
                let target = CellAddress::new(dst_anchor.row + dr, dst_anchor.col + dc);
                let s = &mut self.sheets[sheet.0 as usize];
                match by_offset.get(&(dr, dc)) {
                    Some(cell) => {
                        let new_content = match &cell.content {
                            CellContent::Empty => CellContent::Empty,
                            CellContent::Value(v) => CellContent::Value(v.clone()),
                            CellContent::Formula { ast, .. } => {
                                let shifted = ast.shift(d_row, d_col);
                                CellContent::Formula {
                                    text: shifted.to_formula_string(),
                                    ast: shifted,
                                    cached: None,
                                }
                            }
                        };
                        match new_content {
                            CellContent::Empty => {
                                s.cells.remove(&target);
                            }
                            content => {
                                s.cells.insert(target, Cell { content });
                            }
                        }
                        match &cell.format {
                            Some(code) => {
                                s.formats.insert(target, code.clone());
                            }
                            None => {
                                s.formats.remove(&target);
                            }
                        }
                    }
                    None => {
                        // Blank in the source → erase the target's content + format.
                        s.cells.remove(&target);
                        s.formats.remove(&target);
                    }
                }
            }
        }

        // A cut moves: clear the source cells the paste didn't already overwrite
        // (same sheet AND inside the destination rectangle = already rewritten).
        if clip.is_cut {
            let dst_end_row = dst_anchor.row + clip.rows - 1;
            let dst_end_col = dst_anchor.col + clip.cols - 1;
            for row in clip.source.start.row..=clip.source.end.row {
                for col in clip.source.start.col..=clip.source.end.col {
                    let covered = clip.sheet == sheet
                        && row >= dst_anchor.row
                        && row <= dst_end_row
                        && col >= dst_anchor.col
                        && col <= dst_end_col;
                    if covered {
                        continue;
                    }
                    let src_sheet = &mut self.sheets[clip.sheet.0 as usize];
                    let addr = CellAddress::new(row, col);
                    src_sheet.cells.remove(&addr);
                    src_sheet.formats.remove(&addr);
                }
            }
        }

        // References changed en masse; rebuild edges, recalc once (one revision
        // bump), then log every touched cell so a viewport `changed_since`
        // snapshot taken before the paste sees the moves.
        self.rebuild_dependency_graph();
        self.recalc_all();
        for dr in 0..clip.rows {
            for dc in 0..clip.cols {
                self.log_change(sheet, CellAddress::new(dst_anchor.row + dr, dst_anchor.col + dc));
            }
        }
        if clip.is_cut {
            for row in clip.source.start.row..=clip.source.end.row {
                for col in clip.source.start.col..=clip.source.end.col {
                    self.log_change(clip.sheet, CellAddress::new(row, col));
                }
            }
        }

        // A copy's buffer survives for further pastes; a cut's is consumed.
        if !clip.is_cut {
            self.clipboard = Some(clip);
        }
        true
    }

    /// Whether the clipboard currently holds a copied/cut block.
    pub fn has_clipboard(&self) -> bool {
        self.clipboard.is_some()
    }

    // ----------------------------------------------------------------
    // Range sort (Data ▸ Sort)
    // ----------------------------------------------------------------

    /// Reorder the **rows** of `range` by the computed values in one **key
    /// column** — the engine side of a spreadsheet's *Data ▸ Sort*, and the third
    /// member of the range-operation family (after [`fill`] and the clipboard).
    ///
    /// Each row of `range` is a record spanning the range's columns; the rows are
    /// permuted into key order while every record's cells stay together. The sort
    /// key is the **computed value** at `(row, key_col)` (a formula sorts by what
    /// it evaluates to, not its text), compared under a fixed total order: blanks
    /// always sort last (both directions), otherwise by type — Number < Text <
    /// Boolean < Error — then within a type (numeric, case-insensitive text,
    /// `FALSE`<`TRUE`, fixed error order). `ascending = false` reverses only the
    /// non-empty comparison. The sort is **stable** (equal keys keep their order).
    ///
    /// Because the rows physically move, a moved cell's formula has its references
    /// shifted by that row's displacement (`Δrow`, `Δcol = 0`) via
    /// [`FormulaAst::shift`] — relative refs track, absolute (`$`) refs pin, an
    /// off-grid ref collapses to `#REF!` — exactly as if each row were cut and
    /// pasted to its new position. Display **formats** ride with their cells.
    /// Cells in the sorted rows but *outside* the column band are untouched.
    ///
    /// Returns the **permutation** it applied: `Some(order)` where
    /// `order[new_row_offset] = old_row_offset` (offsets are 0-based from
    /// `range.start.row`), so a caller that keeps its own per-cell side-table —
    /// like the wasm facade's raw-source echo map — can replay the exact same row
    /// move with `rewrite_raw_for_fill` instead of re-deriving the comparator.
    /// `None` is the no-op rejection (unknown `sheet`, `key_col` outside the
    /// range, an empty/inverted/single-row range, or a range over
    /// [`MAX_RANGE_CELLS`] — the shared DoS guard). An already-sorted range
    /// returns `Some(identity)` and is left untouched (no revision bump). One
    /// recalc transaction.
    ///
    /// [`fill`]: Workbook::fill
    /// [`FormulaAst::shift`]: crate::ast::FormulaAst::shift
    pub fn sort_range(
        &mut self,
        sheet: SheetId,
        range: CellRange,
        key_col: u32,
        ascending: bool,
    ) -> Option<Vec<u32>> {
        // Guards: unknown sheet, oversized range (DoS), key column outside the
        // range, or nothing to reorder (empty/inverted/single row).
        self.sheets.get(sheet.0 as usize)?;
        if range.cell_count() > MAX_RANGE_CELLS {
            return None;
        }
        if key_col < range.start.col || key_col > range.end.col {
            return None;
        }
        if range.end.row <= range.start.row {
            return None;
        }

        let first = range.start.row;
        let last = range.end.row;
        let nrows = (last - first + 1) as usize;

        // 1. Read each row's sort key (the computed value at the key column) and
        //    stable-sort the row offsets by it under the documented total order.
        let keys: Vec<CellValue> = (0..nrows)
            .map(|i| self.cell_value(sheet, CellAddress::new(first + i as u32, key_col)))
            .collect();
        let mut order: Vec<usize> = (0..nrows).collect();
        order.sort_by(|&a, &b| Self::compare_sort_keys(&keys[a], &keys[b], ascending));

        // 2. Already in order → don't bump the revision for a no-op, but still
        //    report the (identity) permutation so callers branch uniformly.
        if order.iter().enumerate().all(|(new_i, &old_i)| new_i == old_i) {
            return Some(order.iter().map(|&i| i as u32).collect());
        }

        // 3. Snapshot the whole block before writing — a permutation overwrites
        //    cells in place, so every source must be read up front. Keyed by
        //    (row offset, col), storing the optional content + optional format.
        let s = &self.sheets[sheet.0 as usize];
        let mut snap_content: HashMap<(usize, u32), CellContent> = HashMap::new();
        let mut snap_format: HashMap<(usize, u32), String> = HashMap::new();
        for i in 0..nrows {
            let row = first + i as u32;
            for col in range.start.col..=range.end.col {
                let addr = CellAddress::new(row, col);
                if let Some(cell) = s.cells.get(&addr) {
                    snap_content.insert((i, col), cell.content.clone());
                }
                if let Some(code) = s.formats.get(&addr) {
                    snap_format.insert((i, col), code.clone());
                }
            }
        }

        // 4. Rewrite each destination row from its source row, shifting moved
        //    formulas by the row displacement (Δcol = 0). i64-then-clamp keeps the
        //    subtraction exact for high-coordinate rows (the same guard fill uses).
        for (new_i, &old_i) in order.iter().enumerate() {
            let dest_row = first + new_i as u32;
            let src_row = first + old_i as u32;
            let d_row =
                (dest_row as i64 - src_row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;

            for col in range.start.col..=range.end.col {
                let dest = CellAddress::new(dest_row, col);
                let new_content = match snap_content.get(&(old_i, col)) {
                    None | Some(CellContent::Empty) => CellContent::Empty,
                    Some(CellContent::Value(v)) => CellContent::Value(v.clone()),
                    Some(CellContent::Formula { ast, .. }) => {
                        // `shift_local`, not `shift`: a sort relocates rows *within*
                        // this sheet, so a moved formula's same-sheet refs track the
                        // row displacement, but a cross-sheet (`Summary!A1`) ref names
                        // a fixed cell on another sheet and must not move.
                        let shifted = ast.shift_local(d_row, 0);
                        CellContent::Formula {
                            text: shifted.to_formula_string(),
                            ast: shifted,
                            cached: None, // recomputed by recalc_all
                        }
                    }
                };
                let s = &mut self.sheets[sheet.0 as usize];
                match new_content {
                    CellContent::Empty => {
                        s.cells.remove(&dest);
                    }
                    content => {
                        s.cells.insert(dest, Cell { content });
                    }
                }
                // The format rides with the cell: copy it, or clear the
                // destination's format when the source row had none.
                match snap_format.get(&(old_i, col)) {
                    Some(code) => {
                        s.formats.insert(dest, code.clone());
                    }
                    None => {
                        s.formats.remove(&dest);
                    }
                }
            }
        }

        // 5. References moved en masse; rebuild edges, recalc the whole workbook
        //    (one revision bump), and log every cell in the range so a viewport
        //    `changed_since` snapshot taken before the sort sees the moves.
        self.rebuild_dependency_graph();
        self.recalc_all();
        for i in 0..nrows {
            let row = first + i as u32;
            for col in range.start.col..=range.end.col {
                self.log_change(sheet, CellAddress::new(row, col));
            }
        }
        Some(order.iter().map(|&i| i as u32).collect())
    }

    /// Total order over cell values for [`sort_range`](Workbook::sort_range):
    /// empties always sort last (both directions); otherwise by type rank
    /// (Number < Text < Boolean < Error), then within a type. `ascending = false`
    /// reverses only the non-empty comparison so blanks still sink to the bottom.
    fn compare_sort_keys(a: &CellValue, b: &CellValue, ascending: bool) -> Ordering {
        // Blanks sink to the bottom regardless of direction (Excel's rule), so
        // this is decided *before* the ascending flip at the end.
        match (a.is_empty(), b.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {}
        }
        let ord = match (a, b) {
            (CellValue::Number(x), CellValue::Number(y)) => {
                x.partial_cmp(y).unwrap_or(Ordering::Equal)
            }
            // Case-insensitive primary, case-sensitive tiebreak — so "A" and "a"
            // have a stable, deterministic order rather than comparing equal.
            (CellValue::Text(x), CellValue::Text(y)) => {
                x.to_lowercase().cmp(&y.to_lowercase()).then_with(|| x.cmp(y))
            }
            (CellValue::Boolean(x), CellValue::Boolean(y)) => x.cmp(y),
            (CellValue::Error(x), CellValue::Error(y)) => {
                Self::error_rank(*x).cmp(&Self::error_rank(*y))
            }
            // Different types: order by their type rank.
            _ => Self::type_rank(a).cmp(&Self::type_rank(b)),
        };
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    }

    /// Type ordinal for the cross-type sort order (Number < Text < Boolean <
    /// Error). `Empty` is handled separately (always last) and never reaches here.
    fn type_rank(v: &CellValue) -> u8 {
        match v {
            CellValue::Number(_) => 0,
            CellValue::Text(_) => 1,
            CellValue::Boolean(_) => 2,
            CellValue::Error(_) => 3,
            CellValue::Empty => 4,
        }
    }

    /// A fixed ordinal per error sentinel, so a column of mixed errors sorts
    /// deterministically rather than by hash order.
    fn error_rank(e: SpreadsheetError) -> u8 {
        match e {
            SpreadsheetError::Ref => 0,
            SpreadsheetError::Name => 1,
            SpreadsheetError::DivZero => 2,
            SpreadsheetError::Value => 3,
            SpreadsheetError::NotAvailable => 4,
            SpreadsheetError::Num => 5,
            SpreadsheetError::Null => 6,
            SpreadsheetError::Calc => 7,
            SpreadsheetError::Spill => 8,
            SpreadsheetError::GettingData => 9,
        }
    }

    // ----------------------------------------------------------------
    // Find / replace (Edit ▸ Find / Replace)
    // ----------------------------------------------------------------

    /// Set a cell from a raw user-typed string — the single entry point that owns
    /// the "what a typed string means" policy. Trims, then routes:
    /// empty → [`clear_cell`](Self::clear_cell); a `=`-prefix → [`set_formula`]
    /// (a string that won't parse degrades to a `#VALUE!` literal); otherwise
    /// literal coercion (`"TRUE"`/`"FALSE"` → boolean, a finite number → number,
    /// else text) → [`set_value`]. The replace path and any host can reach the
    /// engine's full cell-entry behaviour through this one call (the facades
    /// previously each re-implemented it).
    ///
    /// [`set_formula`]: Self::set_formula
    /// [`set_value`]: Self::set_value
    pub fn set_raw(&mut self, sheet: SheetId, addr: CellAddress, raw: &str) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            self.clear_cell(sheet, addr);
            return;
        }
        if trimmed.starts_with('=') {
            if self.set_formula(sheet, addr, trimmed).is_err() {
                self.set_value(sheet, addr, CellValue::Error(SpreadsheetError::Value));
            }
            return;
        }
        self.set_value(sheet, addr, coerce_literal(trimmed));
    }

    /// Find every non-empty cell whose text contains `query`, in (row, col) order.
    /// `in_formulas` picks the haystack: the cell's **source** (formula text or a
    /// literal's canonical string) when true, its **computed display** value when
    /// false. `match_case = false` compares case-insensitively (ASCII). An empty
    /// `query` matches nothing. Sparse: scans only populated cells. Unknown sheet
    /// → empty.
    pub fn find_all(
        &self,
        sheet: SheetId,
        query: &str,
        in_formulas: bool,
        match_case: bool,
    ) -> Vec<CellAddress> {
        if query.is_empty() {
            return Vec::new();
        }
        let Some(s) = self.sheets.get(sheet.0 as usize) else {
            return Vec::new();
        };
        let mut hits: Vec<CellAddress> = s
            .cells
            .keys()
            .copied()
            .filter(|&addr| {
                let hay = if in_formulas {
                    self.cell_source_text(sheet, addr)
                } else {
                    self.get_display(sheet, addr)
                };
                contains(&hay, query, match_case)
            })
            .collect();
        // Stable, predictable order for callers (and tests): top-to-bottom,
        // left-to-right.
        hits.sort_by_key(|a| (a.row, a.col));
        hits
    }

    /// Replace `query` with `replacement` in the **source** of every matching
    /// non-empty cell, re-applying each result through [`set_raw`](Self::set_raw)
    /// so the cell re-parses (a still-`=` result as a formula, a literal
    /// re-coerced). `match_case = false` matches case-insensitively. Returns the
    /// number of cells changed. An empty `query` is a no-op (returns 0). Unknown
    /// sheet → 0.
    pub fn replace_all(
        &mut self,
        sheet: SheetId,
        query: &str,
        replacement: &str,
        match_case: bool,
    ) -> usize {
        if query.is_empty() || self.sheets.get(sheet.0 as usize).is_none() {
            return 0;
        }
        // Snapshot the targets first: we mutate cells as we go, and a replacement
        // could in principle reintroduce the query, so decide the work set up front.
        let targets: Vec<(CellAddress, String)> = {
            let s = &self.sheets[sheet.0 as usize];
            s.cells
                .keys()
                .copied()
                .filter_map(|addr| {
                    let src = self.cell_source_text(sheet, addr);
                    if contains(&src, query, match_case) {
                        Some((addr, replace_substring(&src, query, replacement, match_case)))
                    } else {
                        None
                    }
                })
                .collect()
        };
        for (addr, new_raw) in &targets {
            self.set_raw(sheet, *addr, new_raw);
        }
        targets.len()
    }

    /// The cell's **source** text: a formula's stored text, or a literal's
    /// canonical string (the form a user would re-type). Empty cells → "".
    /// Public so a facade can resync its raw-source echo after `replace_all`
    /// rewrites cells (the engine has no other public source accessor).
    pub fn cell_source_text(&self, sheet: SheetId, addr: CellAddress) -> String {
        let Some(s) = self.sheets.get(sheet.0 as usize) else {
            return String::new();
        };
        match s.cells.get(&addr).map(|c| &c.content) {
            Some(CellContent::Formula { text, .. }) => text.clone(),
            Some(CellContent::Value(CellValue::Error(e))) => e.display().to_string(),
            Some(CellContent::Value(v)) => v.coerce_text().unwrap_or_default(),
            Some(CellContent::Empty) | None => String::new(),
        }
    }

    /// Is the cell at `addr` a **formula** (as opposed to a literal value or
    /// empty)? A serializer needs this to decide between emitting a formula and
    /// emitting a plain value: [`cell_source_text`] alone can't tell them apart
    /// (a formula's text and a literal's canonical string are both just strings),
    /// and the `=` prefix is unreliable — the stored text may or may not carry it
    /// depending on how the formula was entered.
    ///
    /// Pairs with [`cell_source_text`] (the formula body) and [`get_value`] (its
    /// cached result) to fully drive a file writer off the unified core model.
    ///
    /// [`cell_source_text`]: Workbook::cell_source_text
    /// [`get_value`]: Workbook::get_value
    pub fn cell_is_formula(&self, sheet: SheetId, addr: CellAddress) -> bool {
        self.sheets
            .get(sheet.0 as usize)
            .and_then(|s| s.cells.get(&addr))
            .map(|c| c.is_formula())
            .unwrap_or(false)
    }

    /// The addresses of every **non-empty** cell on `sheet`, sorted by
    /// `(row, col)`.
    ///
    /// This is the **sparse** counterpart to [`used_range`] (a bounding box): a
    /// serializer must walk only the cells that actually hold content, never the
    /// dense rectangle between them. A sheet with one cell at `A1` and one at
    /// `XFD1048576` has a used range of ~17 billion positions but just two
    /// populated cells — iterating the box would hang; iterating this list is
    /// two steps. Cost is `O(n log n)` in the number of populated cells.
    ///
    /// "Non-empty" matches [`used_range`]'s rule: a present-but-empty cell (e.g.
    /// a formula that evaluated to blank, or a format sitting on an otherwise
    /// empty cell) is excluded.
    ///
    /// [`used_range`]: Workbook::used_range
    pub fn populated_cells(&self, sheet: SheetId) -> Vec<CellAddress> {
        let Some(s) = self.sheets.get(sheet.0 as usize) else {
            return Vec::new();
        };
        let mut addrs: Vec<CellAddress> = s
            .cells
            .iter()
            .filter(|(_, cell)| !cell.current_value().is_empty())
            .map(|(addr, _)| *addr)
            .collect();
        addrs.sort_by_key(|a| (a.row, a.col));
        addrs
    }

    // ----------------------------------------------------------------
    // Persistence — serialize / deserialize (save / load)
    // ----------------------------------------------------------------

    /// Serialize the whole workbook to a portable JSON string — the engine side
    /// of save. Captures every sheet's **source** cells (a formula's text, or a
    /// literal's typed value) plus its **formats** (including formats on
    /// otherwise-empty cells, which outlive content). Computed values are *not*
    /// stored — [`deserialize`] recomputes them, so the file stays small and can
    /// never disagree with the engine.
    ///
    /// Shape (version 1), cells and formats sorted by (row, col) for stable
    /// output. The optional `colWidths` / `rowHeights` arrays (sorted by index)
    /// appear only when a sheet has custom sizes, so a workbook with none is
    /// byte-identical to the pre-feature output:
    /// ```json
    /// {"version":1,"sheets":[{"name":"Sheet1",
    ///   "cells":[{"a1":"A1","value":{"number":15.0}},
    ///            {"a1":"E1","formula":"=SUM(A1:D1)"}],
    ///   "formats":[{"a1":"E1","code":"#,##0.00"}],
    ///   "colWidths":[{"col":3,"w":140.0}],
    ///   "rowHeights":[{"row":2,"h":40.0}]}]}
    /// ```
    /// No I/O happens here — the caller writes the returned string wherever it
    /// likes. Round-trips through [`deserialize`].
    ///
    /// [`deserialize`]: Workbook::deserialize
    pub fn serialize(&self) -> String {
        use serde_json::{json, Value};

        let sheets: Vec<Value> = self
            .sheets
            .iter()
            .map(|s| {
                let mut cells: Vec<(&CellAddress, &Cell)> = s.cells.iter().collect();
                cells.sort_by_key(|(a, _)| (a.row, a.col));
                let cells_json: Vec<Value> = cells
                    .iter()
                    .filter_map(|(addr, cell)| match &cell.content {
                        CellContent::Empty => None,
                        CellContent::Formula { text, .. } => {
                            Some(json!({ "a1": addr.to_a1(), "formula": text }))
                        }
                        CellContent::Value(v) => value_to_json(v)
                            .map(|vj| json!({ "a1": addr.to_a1(), "value": vj })),
                    })
                    .collect();

                let mut fmts: Vec<(&CellAddress, &String)> = s.formats.iter().collect();
                fmts.sort_by_key(|(a, _)| (a.row, a.col));
                let fmts_json: Vec<Value> = fmts
                    .iter()
                    .map(|(addr, code)| json!({ "a1": addr.to_a1(), "code": code }))
                    .collect();

                let mut sheet_obj = json!({
                    "name": s.name, "cells": cells_json, "formats": fmts_json
                });
                // Column widths / row heights are additive: emit them ONLY when a
                // sheet has custom sizes, so a workbook with none serializes
                // byte-identically to the pre-feature output (the document stays
                // version 1, and an old reader ignores unknown keys). Sorted by
                // index for stable output.
                if !s.col_widths.is_empty() {
                    let mut ws: Vec<(&u32, &f64)> = s.col_widths.iter().collect();
                    ws.sort_by_key(|(c, _)| **c);
                    let ws_json: Vec<Value> = ws
                        .iter()
                        .map(|(c, w)| json!({ "col": *c, "w": *w }))
                        .collect();
                    sheet_obj["colWidths"] = Value::Array(ws_json);
                }
                if !s.row_heights.is_empty() {
                    let mut hs: Vec<(&u32, &f64)> = s.row_heights.iter().collect();
                    hs.sort_by_key(|(r, _)| **r);
                    let hs_json: Vec<Value> = hs
                        .iter()
                        .map(|(r, h)| json!({ "row": *r, "h": *h }))
                        .collect();
                    sheet_obj["rowHeights"] = Value::Array(hs_json);
                }
                sheet_obj
            })
            .collect();

        json!({ "version": 1, "sheets": sheets }).to_string()
    }

    /// Replace the workbook's contents from a JSON string produced by
    /// [`serialize`] — the engine side of load. Clears all sheets, the
    /// dependency graph, and the clipboard, rebuilds the sheets in file order
    /// (so a single-sheet host keeps `SheetId(0)`), then `recalc_all` repopulates
    /// every cached value and the revision clock advances once.
    ///
    /// Returns `Err` on malformed JSON, an unsupported `version`, a missing
    /// `sheets` array, or a bad cell address — the workbook is only mutated once
    /// the structure validates (the reset happens after the parse + version
    /// check). A stored formula that no longer parses is kept as its literal text
    /// rather than dropped, so no user input is silently lost.
    ///
    /// No I/O — the caller supplies the bytes it read from wherever.
    ///
    /// [`serialize`]: Workbook::serialize
    pub fn deserialize(&mut self, data: &str) -> Result<(), String> {
        use serde_json::Value;

        let root: Value =
            serde_json::from_str(data).map_err(|e| format!("invalid JSON: {e}"))?;
        match root.get("version").and_then(Value::as_u64) {
            Some(1) => {}
            other => return Err(format!("unsupported workbook version: {other:?}")),
        }
        let sheets = root
            .get("sheets")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing 'sheets' array".to_string())?;

        // Structure validated enough to commit: reset, then rebuild.
        self.sheets.clear();
        self.sheet_by_name.clear();
        self.graph = DependencyGraph::new();
        self.clipboard = None;
        self.changes.clear();

        for sj in sheets {
            let name = sj
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "sheet missing 'name'".to_string())?;
            let sheet = self.add_sheet(name);

            if let Some(cells) = sj.get("cells").and_then(Value::as_array) {
                for c in cells {
                    let a1 = c
                        .get("a1")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "cell missing 'a1'".to_string())?;
                    let addr = CellAddress::parse(a1)
                        .map_err(|e| format!("bad cell address {a1:?}: {}", e.display()))?;
                    if let Some(f) = c.get("formula").and_then(Value::as_str) {
                        // Keep the text as a literal if it no longer parses, so a
                        // saved formula is never silently lost on load.
                        if self.set_formula(sheet, addr, f).is_err() {
                            self.set_value(sheet, addr, CellValue::Text(f.to_string()));
                        }
                    } else if let Some(vj) = c.get("value") {
                        self.set_value(sheet, addr, json_to_value(vj));
                    }
                }
            }

            if let Some(fmts) = sj.get("formats").and_then(Value::as_array) {
                for f in fmts {
                    let a1 = f
                        .get("a1")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "format missing 'a1'".to_string())?;
                    let code = f
                        .get("code")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "format missing 'code'".to_string())?;
                    let addr = CellAddress::parse(a1)
                        .map_err(|e| format!("bad format address {a1:?}: {}", e.display()))?;
                    self.set_format(sheet, addr, code);
                }
            }

            // Column widths / row heights are read TOLERANTLY — a missing array, a
            // non-numeric / non-finite / ≤ 0 value, or a 0 / out-of-u32 index is
            // skipped (never aborts the load), so a hand-edited or future file can't
            // crash a load over presentation chrome. `set_column_width` /
            // `set_row_height` already enforce the finite / `> 0` / index ≥ 1 rules,
            // so a bad entry simply doesn't take.
            if let Some(ws) = sj.get("colWidths").and_then(Value::as_array) {
                for w in ws {
                    if let (Some(col), Some(width)) = (
                        w.get("col").and_then(Value::as_u64),
                        w.get("w").and_then(Value::as_f64),
                    ) {
                        if let Ok(col) = u32::try_from(col) {
                            self.set_column_width(sheet, col, width);
                        }
                    }
                }
            }
            if let Some(hs) = sj.get("rowHeights").and_then(Value::as_array) {
                for h in hs {
                    if let (Some(row), Some(height)) = (
                        h.get("row").and_then(Value::as_u64),
                        h.get("h").and_then(Value::as_f64),
                    ) {
                        if let Ok(row) = u32::try_from(row) {
                            self.set_row_height(sheet, row, height);
                        }
                    }
                }
            }
        }

        // Rebuild the dependency graph now that ALL sheets exist: a cross-sheet
        // formula loaded before its target sheet (sheets load in file order)
        // couldn't resolve its qualifier at `set_formula` time, so its edge was
        // skipped. A full rebuild re-resolves every name → SheetId, registering the
        // cross-sheet edges so a later edit recomputes its dependents.
        self.rebuild_dependency_graph();
        self.recalc_all();
        self.revision = self.revision.wrapping_add(1);
        Ok(())
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
        let sheet_by_name = &self.sheet_by_name;
        let resolve = |name: &str| sheet_by_name.get(name).copied();
        for (i, s) in self.sheets.iter().enumerate() {
            let sheet = SheetId(i as u32);
            for (addr, cell) in &s.cells {
                if let CellContent::Formula { ast, .. } = &cell.content {
                    let mut refs = Vec::new();
                    collect_refs(ast, sheet, &resolve, &mut refs);
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
                // Normalise away `$` markers: a `$A$1` reference must resolve to
                // the same cell as `A1` (cells are keyed by position only).
                sheets
                    .get(sid.0 as usize)
                    .and_then(|s| s.cells.get(&a.without_absolute()))
                    .map(|c| c.current_value())
                    .unwrap_or(CellValue::Empty)
            };
            // Map a sheet name to its id so a cross-sheet ref (`Summary!A1`) reads
            // the target sheet; an unknown name resolves to `None` → `#REF!`.
            let sheet_by_name = &self.sheet_by_name;
            let resolve = |name: &str| sheet_by_name.get(name).copied();
            match evaluate(&ast, sheet, &lookup, &resolve) {
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

/// Validate a 1-based inclusive window request and return its `(rows, cols)`
/// dimensions in `u64`. Shared by [`Workbook::get_window`] and
/// [`Workbook::get_display_window`] so the bounds + overflow + cap checks can't
/// drift apart.
///
/// Rejects (`#REF!`) a 0 coordinate (out of the 1-based contract — and a `row0`/
/// `col0` of 0 would let the span computation cover the full u32 range), an
/// inverted rectangle, and a window over [`MAX_WINDOW_CELLS`]. The span operands
/// are widened to `u64` **before** the `+ 1` — `(row1 - row0 + 1)` in `u32` would
/// overflow when `row1 - row0 == u32::MAX`, wrapping to a bogus small count that
/// slips past the cap and sends the caller's loop over the entire u32 range (an
/// OOM DoS). `checked_mul` then rejects any product that still overflows `u64`.
/// Re-key a column-width / row-height map under a structural edit: apply `f` to
/// every key, keeping the entry at its new key when `f` returns `Some` and dropping
/// it when `f` returns `None` (the index sat inside a deleted band). The value (the
/// width / height) rides along unchanged. Used by [`Workbook::apply_structural_edit`]
/// to slide column widths / row heights with their columns / rows.
fn shift_axis_keys(
    map: HashMap<u32, f64>,
    f: impl Fn(u32) -> Option<u32>,
) -> HashMap<u32, f64> {
    map.into_iter()
        .filter_map(|(k, v)| f(k).map(|nk| (nk, v)))
        .collect()
}

/// Coerce a raw literal string (already trimmed, not a formula) to a typed value:
/// `"TRUE"`/`"FALSE"` (any case) → boolean, a finite parseable number → number,
/// anything else → text. The literal half of [`Workbook::set_raw`]'s policy.
fn coerce_literal(s: &str) -> CellValue {
    match s.to_ascii_uppercase().as_str() {
        "TRUE" => return CellValue::Boolean(true),
        "FALSE" => return CellValue::Boolean(false),
        _ => {}
    }
    if let Ok(n) = s.parse::<f64>() {
        if n.is_finite() {
            return CellValue::Number(n);
        }
    }
    CellValue::Text(s.to_string())
}

/// Substring containment, honoring `match_case`. Case-insensitive uses an ASCII
/// lowercase fold (the spreadsheet convention; full Unicode case-folding is out of
/// scope, matching the rest of the engine's ASCII-oriented text handling).
fn contains(haystack: &str, needle: &str, match_case: bool) -> bool {
    if match_case {
        haystack.contains(needle)
    } else {
        haystack
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
    }
}

/// Replace every occurrence of `needle` with `repl` in `haystack`. Case-sensitive
/// uses `str::replace`; case-insensitive scans the ASCII-lowercased haystack for
/// match positions and splices `repl` over the original (case-preserving) spans.
/// `needle` is always non-empty here (callers guard the empty query).
fn replace_substring(haystack: &str, needle: &str, repl: &str, match_case: bool) -> String {
    if match_case {
        return haystack.replace(needle, repl);
    }
    let hay_lower = haystack.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < haystack.len() {
        if hay_lower[i..].starts_with(&needle_lower) {
            out.push_str(repl);
            i += needle.len();
        } else {
            // Advance one full UTF-8 char so we never split a multibyte boundary.
            let ch = haystack[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn window_dims(row0: u32, col0: u32, row1: u32, col1: u32) -> Result<(u64, u64), SpreadsheetError> {
    if row0 == 0 || col0 == 0 || row1 < row0 || col1 < col0 {
        return Err(SpreadsheetError::Ref);
    }
    let rows = (row1 as u64 - row0 as u64) + 1;
    let cols = (col1 as u64 - col0 as u64) + 1;
    match rows.checked_mul(cols) {
        Some(n) if n <= MAX_WINDOW_CELLS => Ok((rows, cols)),
        _ => Err(SpreadsheetError::Ref),
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

/// Encode a literal [`CellValue`] for [`Workbook::serialize`]. `Empty` returns
/// `None` (nothing to store). A non-finite number can't be represented in JSON,
/// so it degrades to its `#NUM!` error sentinel — the same way the value would
/// read in the grid.
fn value_to_json(v: &CellValue) -> Option<serde_json::Value> {
    use serde_json::json;
    match v {
        CellValue::Empty => None,
        CellValue::Boolean(b) => Some(json!({ "bool": b })),
        CellValue::Number(n) if n.is_finite() => Some(json!({ "number": n })),
        CellValue::Number(_) => Some(json!({ "error": SpreadsheetError::Num.display() })),
        CellValue::Text(s) => Some(json!({ "text": s })),
        CellValue::Error(e) => Some(json!({ "error": e.display() })),
    }
}

/// Decode a `value` object from [`Workbook::deserialize`] back into a
/// [`CellValue`]. Unknown / malformed shapes fall back to `Empty` rather than
/// failing the whole load — a single odd cell shouldn't lose the rest of the
/// sheet.
fn json_to_value(vj: &serde_json::Value) -> CellValue {
    if let Some(b) = vj.get("bool").and_then(serde_json::Value::as_bool) {
        CellValue::Boolean(b)
    } else if let Some(n) = vj.get("number").and_then(serde_json::Value::as_f64) {
        CellValue::Number(n)
    } else if let Some(t) = vj.get("text").and_then(serde_json::Value::as_str) {
        CellValue::Text(t.to_string())
    } else if let Some(code) = vj.get("error").and_then(serde_json::Value::as_str) {
        error_from_code(code).map_or_else(|| CellValue::Text(code.to_string()), CellValue::Error)
    } else {
        CellValue::Empty
    }
}

/// Reverse of [`SpreadsheetError::display`] — map a sentinel string back to its
/// variant. Returns `None` for an unrecognised code.
fn error_from_code(code: &str) -> Option<SpreadsheetError> {
    Some(match code {
        "#REF!" => SpreadsheetError::Ref,
        "#NAME?" => SpreadsheetError::Name,
        "#DIV/0!" => SpreadsheetError::DivZero,
        "#VALUE!" => SpreadsheetError::Value,
        "#N/A" => SpreadsheetError::NotAvailable,
        "#NUM!" => SpreadsheetError::Num,
        "#NULL!" => SpreadsheetError::Null,
        "#CALC!" => SpreadsheetError::Calc,
        "#SPILL!" => SpreadsheetError::Spill,
        "#GETTING_DATA" => SpreadsheetError::GettingData,
        _ => return None,
    })
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
    fn cross_sheet_formula_reads_and_recomputes_across_sheets() {
        // Two sheets: Sheet1 holds a formula that references Summary!A1. Editing
        // Summary's cell must recompute Sheet1's dependent through the cross-sheet
        // dependency graph — the payoff of resolving the qualifier to a SheetId in
        // both eval and dependency collection.
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(1, 1), CellValue::Number(10.0)); // Summary!A1 = 10
        wb.set_formula(s1, cell(1, 2), "=Summary!A1*2").unwrap(); // Sheet1!B1
        assert_eq!(wb.cell_value(s1, cell(1, 2)), CellValue::Number(20.0));

        // Edit the precedent on the OTHER sheet → the cross-sheet dependent updates.
        wb.set_value(summary, cell(1, 1), CellValue::Number(50.0));
        assert_eq!(wb.cell_value(s1, cell(1, 2)), CellValue::Number(100.0));

        // A qualified SUM over a range on another sheet works too.
        wb.set_value(summary, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(summary, cell(2, 1), CellValue::Number(2.0));
        wb.set_value(summary, cell(3, 1), CellValue::Number(3.0));
        wb.set_formula(s1, cell(2, 2), "=SUM(Summary!A1:A3)").unwrap();
        assert_eq!(wb.cell_value(s1, cell(2, 2)), CellValue::Number(6.0));

        // A reference to a sheet that doesn't exist is #REF! (not a panic).
        wb.set_formula(s1, cell(3, 2), "=Ghost!A1").unwrap();
        assert_eq!(
            wb.cell_value(s1, cell(3, 2)),
            CellValue::Error(SpreadsheetError::Ref)
        );

        // The cross-sheet qualifier is preserved in the cell's stored source.
        wb.set_formula(s1, cell(4, 2), "=Summary!A1+1").unwrap();
        assert_eq!(wb.cell_source_text(s1, cell(4, 2)), "=Summary!A1+1");
    }

    #[test]
    fn structural_edit_propagates_into_inbound_cross_sheet_refs() {
        // Sheet1!B1 = =Summary!A5*1; Summary!A5 = 99. Inserting a row ABOVE row 5
        // on Summary pushes its A5 down to A6, and the inbound `Summary!A5` ref on
        // Sheet1 must follow to `Summary!A6` (and keep computing 99).
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(5, 1), CellValue::Number(99.0)); // Summary!A5
        wb.set_formula(s1, cell(1, 2), "=Summary!A5*1").unwrap(); // Sheet1!B1
        assert_eq!(wb.cell_value(s1, cell(1, 2)), CellValue::Number(99.0));

        wb.insert_rows(summary, 3, 1); // insert above row 5 → A5 slides to A6
        // A structural edit re-emits the source from the AST (no leading `=`,
        // fully parenthesised) — the inbound ref now names Summary!A6.
        assert_eq!(wb.cell_source_text(s1, cell(1, 2)), "(Summary!A6*1)");
        assert_eq!(wb.cell_value(s1, cell(1, 2)), CellValue::Number(99.0));

        // Deleting the band the inbound ref points at turns it into #REF!.
        wb.delete_rows(summary, 6, 1); // delete the row holding the (moved) value
        assert_eq!(
            wb.cell_value(s1, cell(1, 2)),
            CellValue::Error(SpreadsheetError::Ref)
        );

        // A structural edit on Sheet1 does NOT disturb its OWN cross-sheet refs
        // (they point into Summary, which wasn't edited): re-seed and insert on s1.
        wb.set_value(summary, cell(1, 1), CellValue::Number(5.0));
        wb.set_formula(s1, cell(10, 2), "=Summary!A1").unwrap();
        wb.insert_rows(s1, 1, 1); // moves the formula down a row but leaves its ref
        assert_eq!(wb.cell_source_text(s1, cell(11, 2)), "Summary!A1");
        assert_eq!(wb.cell_value(s1, cell(11, 2)), CellValue::Number(5.0));
    }

    #[test]
    fn rename_sheet_rewrites_qualifiers_and_keeps_values() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(1, 1), CellValue::Number(7.0));
        wb.set_formula(s1, cell(1, 1), "=Summary!A1*2").unwrap(); // 14
        assert_eq!(wb.cell_value(s1, cell(1, 1)), CellValue::Number(14.0));

        // Rename Summary → Totals: the qualifier in Sheet1's formula follows, the
        // value is unchanged, and the new name resolves while the old does not.
        wb.rename_sheet(summary, "Totals").unwrap();
        assert_eq!(wb.sheet_names(), vec!["Sheet1", "Totals"]);
        // Rename re-emits the source from the AST (no leading `=`, parenthesised).
        assert_eq!(wb.cell_source_text(s1, cell(1, 1)), "(Totals!A1*2)");
        assert_eq!(wb.cell_value(s1, cell(1, 1)), CellValue::Number(14.0));
        // Editing the renamed sheet still recomputes the dependent (deps intact).
        wb.set_value(summary, cell(1, 1), CellValue::Number(10.0));
        assert_eq!(wb.cell_value(s1, cell(1, 1)), CellValue::Number(20.0));

        // Guards: empty name and a duplicate of another sheet are rejected.
        assert!(wb.rename_sheet(summary, "").is_err());
        assert!(wb.rename_sheet(summary, "Sheet1").is_err());
    }

    #[test]
    fn delete_sheet_reindexes_and_makes_inbound_refs_ref_error() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let mid = wb.add_sheet("Mid");
        let last = wb.add_sheet("Last");
        wb.set_value(last, cell(1, 1), CellValue::Number(5.0));
        wb.set_value(mid, cell(1, 1), CellValue::Number(9.0));
        wb.set_formula(s1, cell(1, 1), "=Mid!A1").unwrap(); // → 9
        wb.set_formula(s1, cell(2, 1), "=Last!A1").unwrap(); // → 5
        assert_eq!(wb.cell_value(s1, cell(1, 1)), CellValue::Number(9.0));

        // Delete the middle sheet: Last's SheetId shifts from 2 → 1, the name index
        // follows, the inbound `=Mid!A1` ref becomes #REF!, and `=Last!A1` still
        // resolves (by NAME) to the reindexed sheet.
        wb.delete_sheet(mid).unwrap();
        assert_eq!(wb.sheet_names(), vec!["Sheet1", "Last"]);
        assert_eq!(wb.sheet_id("Last"), Some(SheetId(1)));
        assert_eq!(
            wb.cell_value(s1, cell(1, 1)),
            CellValue::Error(SpreadsheetError::Ref)
        );
        assert_eq!(wb.cell_value(s1, cell(2, 1)), CellValue::Number(5.0));
        // Re-adding a sheet named Mid does NOT resurrect the dead ref (now #REF!).
        let _ = wb.add_sheet("Mid");
        assert_eq!(
            wb.cell_value(s1, cell(1, 1)),
            CellValue::Error(SpreadsheetError::Ref)
        );

        // Can't delete the last remaining sheet.
        let mut single = Workbook::new();
        let only = single.add_sheet("Only");
        assert!(single.delete_sheet(only).is_err());
    }

    #[test]
    fn move_sheet_reorders_tabs_and_preserves_cross_sheet_values() {
        let mut wb = Workbook::new();
        let a = wb.add_sheet("A");
        let b = wb.add_sheet("B");
        let _c = wb.add_sheet("C");
        wb.set_value(b, cell(1, 1), CellValue::Number(8.0));
        wb.set_formula(a, cell(1, 1), "=B!A1+1").unwrap(); // 9
        // Move A to the end: tab order becomes B, C, A; the cross-sheet value holds
        // (refs resolve by name, not by the shifted id).
        wb.move_sheet(a, 2).unwrap();
        assert_eq!(wb.sheet_names(), vec!["B", "C", "A"]);
        let a_now = wb.sheet_id("A").unwrap();
        assert_eq!(wb.cell_value(a_now, cell(1, 1)), CellValue::Number(9.0));
        wb.set_value(wb.sheet_id("B").unwrap(), cell(1, 1), CellValue::Number(20.0));
        assert_eq!(wb.cell_value(a_now, cell(1, 1)), CellValue::Number(21.0));
    }

    #[test]
    fn serialize_round_trips_multiple_sheets_and_cross_sheet_refs() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(1, 1), CellValue::Number(50.0));
        wb.set_formula(s1, cell(1, 1), "=Summary!A1+1").unwrap(); // 51
        let doc = wb.serialize();

        let mut loaded = Workbook::new();
        loaded.deserialize(&doc).unwrap();
        assert_eq!(loaded.sheet_names(), vec!["Sheet1", "Summary"]);
        let ls1 = loaded.sheet_id("Sheet1").unwrap();
        // The cross-sheet formula reloaded live: it recomputes against Summary.
        assert_eq!(loaded.cell_value(ls1, cell(1, 1)), CellValue::Number(51.0));
        loaded.set_value(loaded.sheet_id("Summary").unwrap(), cell(1, 1), CellValue::Number(99.0));
        assert_eq!(loaded.cell_value(ls1, cell(1, 1)), CellValue::Number(100.0));
    }

    #[test]
    fn fill_shifts_cross_sheet_relative_refs_keeping_the_qualifier() {
        // Drag-fill replicates a formula and shifts its relative refs by the copy
        // offset — including a qualified relative ref. Filling =Summary!A1 down a
        // column gives =Summary!A2, =Summary!A3, … (the qualifier rides along).
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(1, 1), CellValue::Number(11.0)); // Summary!A1
        wb.set_value(summary, cell(2, 1), CellValue::Number(22.0)); // Summary!A2
        wb.set_value(summary, cell(3, 1), CellValue::Number(33.0)); // Summary!A3
        wb.set_formula(s1, cell(1, 1), "=Summary!A1").unwrap(); // Sheet1!A1
        // Fill A1 down into A2:A3.
        wb.fill(s1, cell(1, 1), CellRange::new(cell(2, 1), cell(3, 1)));
        assert_eq!(wb.cell_source_text(s1, cell(2, 1)), "Summary!A2");
        assert_eq!(wb.cell_source_text(s1, cell(3, 1)), "Summary!A3");
        assert_eq!(wb.cell_value(s1, cell(2, 1)), CellValue::Number(22.0));
        assert_eq!(wb.cell_value(s1, cell(3, 1)), CellValue::Number(33.0));
    }

    #[test]
    fn sort_does_not_shift_cross_sheet_refs() {
        // Two-row block on Sheet1 where each row's B holds a cross-sheet ref to a
        // FIXED Summary cell. Sorting the block by column A (descending) reorders
        // the rows, but the `Summary!…` refs must stay pinned (they name cells on
        // another sheet that didn't move).
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let summary = wb.add_sheet("Summary");
        wb.set_value(summary, cell(1, 1), CellValue::Number(100.0)); // Summary!A1
        wb.set_value(s1, cell(1, 1), CellValue::Number(1.0)); // A1 key
        wb.set_value(s1, cell(2, 1), CellValue::Number(2.0)); // A2 key
        wb.set_formula(s1, cell(1, 2), "=Summary!A1").unwrap(); // B1
        wb.set_formula(s1, cell(2, 2), "=Summary!A1+1").unwrap(); // B2
        // Sort A1:B2 by column A descending → rows swap (2 then 1).
        wb.sort_range(s1, CellRange::new(cell(1, 1), cell(2, 2)), 1, false);
        // The cross-sheet refs travelled with their row but did NOT shift address
        // (sort re-emits from the AST, so no leading `=`).
        assert_eq!(wb.cell_source_text(s1, cell(1, 2)), "(Summary!A1+1)"); // was B2
        assert_eq!(wb.cell_source_text(s1, cell(2, 2)), "Summary!A1"); // was B1
        assert_eq!(wb.cell_value(s1, cell(1, 2)), CellValue::Number(101.0));
        assert_eq!(wb.cell_value(s1, cell(2, 2)), CellValue::Number(100.0));
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
    fn get_display_window_renders_formatted_strings() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1234.5));
        wb.set_format(s, cell(1, 1), "#,##0.00");
        wb.set_value(s, cell(1, 2), CellValue::Number(0.5));
        wb.set_format(s, cell(1, 2), "0%");
        wb.set_value(s, cell(2, 1), CellValue::Text("hi".into())); // unformatted
        // A1=formatted, B1=percent, A2=text, B2=empty — row-major.
        let dw = wb.get_display_window(s, 1, 1, 2, 2).unwrap();
        assert_eq!((dw.rows, dw.cols), (2, 2));
        assert_eq!(
            dw.cells,
            vec![
                "1,234.50".to_string(),
                "50%".to_string(),
                "hi".to_string(),
                String::new(),
            ]
        );
        // Same guards as get_window.
        assert!(wb.get_display_window(s, 0, 1, 1, 1).is_err()); // 0 coord
        assert!(wb.get_display_window(s, 2, 1, 1, 1).is_err()); // inverted
        assert!(wb.get_display_window(s, 1, 1, 400, 400).is_err()); // oversized
        assert!(wb.get_display_window(SheetId(9), 1, 1, 1, 1).is_err()); // bad sheet
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

    // ── Absolute-reference resolution (regression) ───────────────────

    #[test]
    fn absolute_references_resolve_to_the_same_cell_as_relative() {
        // A cell is keyed by position only, so $A$1 / $A1 / A$1 / A1 in a formula
        // must all read the value at A1. (Regression: the evaluator once keyed the
        // cell lookup by the reference's full address incl. its `$` flags, so an
        // absolute reference missed the relatively-stored cell and read 0.)
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(7.0)); // A1
        wb.set_formula(s, cell(1, 2), "=$A$1+$A1+A$1+A1").unwrap(); // B1 = 7*4
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(28.0)));
        // And the absolute precedent is tracked: editing A1 recomputes B1.
        wb.set_value(s, cell(1, 1), CellValue::Number(10.0));
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(40.0)));
        // An absolute range corner resolves too: SUM($A$1:$A$2).
        wb.set_value(s, cell(2, 1), CellValue::Number(5.0)); // A2
        wb.set_formula(s, cell(1, 3), "=SUM($A$1:$A$2)").unwrap(); // C1 = 15
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(15.0)));
    }

    // ── Fill / replicate ─────────────────────────────────────────────

    #[test]
    fn fill_shifts_relative_formula_references_down_a_column() {
        // Classic cross-foot: B1=A1*2, then fill B1 down B2:B4 over a column of
        // inputs. Each filled formula tracks its row: B2 = A2*2, etc.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, v) in [(1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0)] {
            wb.set_value(s, cell(r, 1), CellValue::Number(v)); // A1..A4
        }
        wb.set_formula(s, cell(1, 2), "=A1*2").unwrap(); // B1 = 20
        wb.fill(s, cell(1, 2), CellRange::new(cell(2, 2), cell(4, 2))); // fill B2:B4

        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Number(40.0))); // A2*2
        assert_eq!(wb.get_value(s, cell(3, 2)), Some(CellValue::Number(60.0))); // A3*2
        assert_eq!(wb.get_value(s, cell(4, 2)), Some(CellValue::Number(80.0))); // A4*2
    }

    #[test]
    fn fill_pins_absolute_references() {
        // B1 = A1 * $A$1 ; filled down, the relative A1 tracks but $A$1 stays.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(3.0)); // A1
        wb.set_value(s, cell(2, 1), CellValue::Number(5.0)); // A2
        wb.set_formula(s, cell(1, 2), "=A1*$A$1").unwrap(); // B1 = 9
        wb.fill(s, cell(1, 2), CellRange::new(cell(2, 2), cell(2, 2))); // fill B2

        // B2 = A2 * $A$1 = 5 * 3 = 15 (the absolute corner stayed at A1).
        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Number(15.0)));
    }

    #[test]
    fn fill_copies_literals_and_formats() {
        // A literal source replicates unchanged, and its display format rides along.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1234.5));
        wb.set_format(s, cell(1, 1), "#,##0.00");
        wb.fill(s, cell(1, 1), CellRange::new(cell(1, 2), cell(1, 3))); // fill right B1:C1

        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(1234.5)));
        assert_eq!(wb.get_format(s, cell(1, 3)), Some("#,##0.00"));
        assert_eq!(wb.get_display(s, cell(1, 2)), "1,234.50"); // format applied
    }

    #[test]
    fn fill_off_grid_reference_becomes_ref_error() {
        // B2 = A1 (one row up-left of B2). Fill it up into B1, where the relative
        // ref would point at row 0 → #REF!.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(7.0)); // A1
        wb.set_formula(s, cell(2, 2), "=A1").unwrap(); // B2 = 7
        wb.fill(s, cell(2, 2), CellRange::new(cell(1, 2), cell(1, 2))); // fill up into B1

        assert_eq!(
            wb.get_value(s, cell(1, 2)),
            Some(CellValue::Error(SpreadsheetError::Ref))
        );
    }

    #[test]
    fn fill_from_empty_source_clears_targets() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(2, 1), CellValue::Number(9.0)); // A2 set
        // A1 was never set (empty). Filling it over A2 clears A2.
        wb.fill(s, cell(1, 1), CellRange::new(cell(2, 1), cell(2, 1)));
        assert_eq!(wb.get_value(s, cell(2, 1)), None);
    }

    #[test]
    fn fill_oversized_range_is_rejected_wholesale() {
        // A fill spanning more than MAX_RANGE_CELLS is a no-op (DoS guard), so a
        // pre-existing cell in that range is left untouched rather than overwritten.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(5, 5), CellValue::Number(42.0));
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        // 1 .. 2^20 rows in one column is exactly the cap; add a row to exceed it.
        let huge = CellRange::new(cell(1, 1), cell((1 << 20) + 1, 1));
        assert!(huge.cell_count() > MAX_RANGE_CELLS);
        wb.fill(s, cell(1, 1), huge);
        assert_eq!(wb.get_value(s, cell(5, 5)), Some(CellValue::Number(42.0))); // untouched
    }

    #[test]
    fn fill_at_high_coordinate_does_not_overflow() {
        // A single-cell fill anchored near u32::MAX passes the cell_count guard
        // (count = 1) but its offset arithmetic must not overflow i32 (would
        // panic in debug/test, wrap in release). Filling a literal far down the
        // sheet just copies it; a formula's huge negative shift collapses to #REF!.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        let hi = 2_147_483_648; // > i32::MAX
        wb.set_value(s, cell(hi, 1), CellValue::Number(5.0));
        // Fill that literal one row further down — no panic, value copied.
        wb.fill(s, cell(hi, 1), CellRange::new(cell(hi + 1, 1), cell(hi + 1, 1)));
        assert_eq!(wb.get_value(s, cell(hi + 1, 1)), Some(CellValue::Number(5.0)));
        // A formula at the high anchor filled back to row 1 shifts by a huge
        // negative delta → its reference goes off-grid → #REF! (no overflow).
        wb.set_formula(s, cell(hi, 2), "=A1").unwrap();
        wb.fill(s, cell(hi, 2), CellRange::new(cell(1, 2), cell(1, 2)));
        assert_eq!(
            wb.get_value(s, cell(1, 2)),
            Some(CellValue::Error(SpreadsheetError::Ref))
        );
    }

    #[test]
    fn fill_unknown_sheet_is_noop() {
        let mut wb = Workbook::new();
        wb.fill(SheetId(9), cell(1, 1), CellRange::new(cell(1, 1), cell(2, 2)));
        // No panic, nothing created.
        assert_eq!(wb.sheet_count(), 0);
    }

    // ── Clipboard: cut / copy / paste ────────────────────────────────

    #[test]
    fn copy_paste_shifts_a_block_as_a_unit() {
        // A 1×2 block B1:C1 where C1 = B1*2. Copy it and paste at B2 → the block
        // moves down one row: B2 keeps its literal, C2 = B2*2 (the relative ref
        // tracked the whole-block shift).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 2), CellValue::Number(5.0)); // B1 = 5
        wb.set_formula(s, cell(1, 3), "=B1*2").unwrap(); // C1 = 10
        wb.copy(s, CellRange::new(cell(1, 2), cell(1, 3))); // copy B1:C1
        assert!(wb.has_clipboard());
        assert!(wb.paste(s, cell(2, 2))); // paste at B2

        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Number(5.0))); // B2
        assert_eq!(wb.get_value(s, cell(2, 3)), Some(CellValue::Number(10.0))); // C2 = B2*2
        // Source is untouched and a copy's buffer survives for another paste.
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(5.0)));
        assert!(wb.has_clipboard());
        assert!(wb.paste(s, cell(3, 2))); // paste again at B3
        assert_eq!(wb.get_value(s, cell(3, 3)), Some(CellValue::Number(10.0))); // C3 = B3*2
    }

    #[test]
    fn copy_carries_format_and_pins_absolute_refs() {
        // A1 = 1234.5 formatted; B1 = =$A$1 (absolute). Copy A1:B1 and paste two
        // rows down at A3: the format rides along, and the absolute ref stays
        // pinned to $A$1 (does NOT shift), so B3 still reads A1's value.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1234.5));
        wb.set_format(s, cell(1, 1), "#,##0.00");
        wb.set_formula(s, cell(1, 2), "=$A$1").unwrap(); // B1 → 1234.5
        wb.copy(s, CellRange::new(cell(1, 1), cell(1, 2)));
        assert!(wb.paste(s, cell(3, 1))); // paste at A3

        assert_eq!(wb.get_display(s, cell(3, 1)), "1,234.50"); // format carried
        // Absolute ref pinned: B3 still points at $A$1 (1234.5), not A3.
        assert_eq!(wb.get_value(s, cell(3, 2)), Some(CellValue::Number(1234.5)));
    }

    #[test]
    fn paste_clears_blank_cells_of_the_source_rectangle() {
        // Copy a 1×2 block whose second cell is blank over a destination whose
        // matching cell is occupied — the blank must erase the target (a paste
        // overwrites the whole rectangle, not just the non-blank cells).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(7.0)); // A1 = 7, B1 blank
        wb.set_value(s, cell(3, 2), CellValue::Number(99.0)); // B3 occupied (target)
        wb.copy(s, CellRange::new(cell(1, 1), cell(1, 2))); // copy A1:B1
        assert!(wb.paste(s, cell(3, 1))); // paste at A3:B3

        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(7.0))); // A3 = 7
        assert_eq!(wb.get_value(s, cell(3, 2)), None); // B3 erased by the blank
    }

    #[test]
    fn cut_paste_moves_and_clears_the_source() {
        // Cut A1 (=5) and paste at C1. The value moves; A1 is cleared; the buffer
        // is consumed (a cut pastes once).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(5.0));
        wb.set_format(s, cell(1, 1), "#,##0.00");
        wb.cut(s, CellRange::new(cell(1, 1), cell(1, 1)));
        assert!(wb.paste(s, cell(1, 3))); // paste at C1

        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Number(5.0))); // moved
        assert_eq!(wb.get_format(s, cell(1, 3)), Some("#,##0.00")); // format moved too
        assert_eq!(wb.get_value(s, cell(1, 1)), None); // source cleared
        assert_eq!(wb.get_format(s, cell(1, 1)), None);
        // Buffer consumed: a second paste does nothing.
        assert!(!wb.has_clipboard());
        assert!(!wb.paste(s, cell(1, 5)));
        assert_eq!(wb.get_value(s, cell(1, 5)), None);
    }

    #[test]
    fn paste_off_grid_is_rejected_and_keeps_the_buffer() {
        // A 1×2 copy whose destination's second column would run past the last
        // column (u32::MAX) is rejected wholesale — nothing is written and the
        // buffer is preserved.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(1, 2), CellValue::Number(2.0));
        wb.copy(s, CellRange::new(cell(1, 1), cell(1, 2))); // 1×2 block
        // Anchor at the last column: the second cell would be at col u32::MAX+1.
        assert!(!wb.paste(s, cell(1, u32::MAX)));
        assert!(wb.has_clipboard()); // buffer kept for a valid paste
        assert_eq!(wb.get_value(s, cell(1, u32::MAX)), None); // nothing written
    }

    #[test]
    fn copy_oversized_range_captures_nothing() {
        // A copy spanning more than MAX_RANGE_CELLS is rejected (DoS guard), so
        // the clipboard stays empty and a following paste is a no-op.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        let huge = CellRange::new(cell(1, 1), cell((1 << 20) + 1, 1));
        assert!(huge.cell_count() > MAX_RANGE_CELLS);
        wb.copy(s, huge);
        assert!(!wb.has_clipboard());
        assert!(!wb.paste(s, cell(10, 10)));
    }

    #[test]
    fn paste_with_empty_clipboard_is_noop() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        assert!(!wb.has_clipboard());
        assert!(!wb.paste(s, cell(1, 1)));
    }

    #[test]
    fn copy_unknown_sheet_is_noop() {
        let mut wb = Workbook::new();
        wb.copy(SheetId(9), CellRange::new(cell(1, 1), cell(2, 2)));
        assert!(!wb.has_clipboard());
    }

    // ── Persistence: serialize / deserialize ─────────────────────────

    #[test]
    fn serialize_then_deserialize_round_trips_values_formulas_and_formats() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("Sheet1");
        wb.set_value(s, cell(1, 1), CellValue::Number(15.0)); // A1
        wb.set_value(s, cell(1, 2), CellValue::Text("hi".into())); // B1
        wb.set_value(s, cell(1, 3), CellValue::Boolean(true)); // C1
        wb.set_formula(s, cell(2, 1), "=A1*2").unwrap(); // A2 = 30
        wb.set_format(s, cell(1, 1), "#,##0.00");
        wb.set_format(s, cell(9, 9), "0%"); // a format on an otherwise-empty cell

        let saved = wb.serialize();

        // Load into a fresh workbook and confirm every cell recomputed identically.
        let mut loaded = Workbook::new();
        loaded.deserialize(&saved).unwrap();
        let s2 = loaded.sheet_id("Sheet1").unwrap();
        assert_eq!(loaded.get_value(s2, cell(1, 1)), Some(CellValue::Number(15.0)));
        assert_eq!(loaded.get_value(s2, cell(1, 2)), Some(CellValue::Text("hi".into())));
        assert_eq!(loaded.get_value(s2, cell(1, 3)), Some(CellValue::Boolean(true)));
        assert_eq!(loaded.get_value(s2, cell(2, 1)), Some(CellValue::Number(30.0))); // formula recomputed
        assert_eq!(loaded.get_format(s2, cell(1, 1)), Some("#,##0.00"));
        assert_eq!(loaded.get_format(s2, cell(9, 9)), Some("0%")); // empty-cell format survived
        // The formula re-evaluates against the loaded inputs (not a frozen value).
        loaded.set_value(s2, cell(1, 1), CellValue::Number(100.0));
        assert_eq!(loaded.get_value(s2, cell(2, 1)), Some(CellValue::Number(200.0)));
        // Serializing the loaded workbook yields byte-identical JSON (stable order).
        assert_eq!(wb.serialize(), saved);
    }

    #[test]
    fn deserialize_replaces_existing_contents() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("Sheet1");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        let saved = wb.serialize(); // a sheet with only A1=1

        // Pre-fill a different workbook, then load over it.
        let mut other = Workbook::new();
        let os = other.add_sheet("Sheet1");
        other.set_value(os, cell(5, 5), CellValue::Number(999.0)); // E5 — should vanish
        other.deserialize(&saved).unwrap();
        let s2 = other.sheet_id("Sheet1").unwrap();
        assert_eq!(other.get_value(s2, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(other.get_value(s2, cell(5, 5)), None); // replaced, not merged
        assert_eq!(other.sheet_count(), 1);
    }

    #[test]
    fn deserialize_rejects_bad_json_and_bad_version() {
        let mut wb = Workbook::new();
        wb.add_sheet("Sheet1");
        assert!(wb.deserialize("not json").is_err());
        assert!(wb.deserialize(r#"{"version":99,"sheets":[]}"#).is_err());
        assert!(wb.deserialize(r#"{"version":1}"#).is_err()); // missing sheets
        // A valid empty workbook loads fine (zero sheets).
        assert!(wb.deserialize(r#"{"version":1,"sheets":[]}"#).is_ok());
        assert_eq!(wb.sheet_count(), 0);
    }

    #[test]
    fn deserialize_keeps_an_unparseable_formula_as_text() {
        // A corrupt/old formula that no longer parses is preserved as literal
        // text rather than silently dropped.
        let mut wb = Workbook::new();
        let json = r#"{"version":1,"sheets":[{"name":"Sheet1",
            "cells":[{"a1":"A1","formula":"=THIS IS NOT A FORMULA"}],"formats":[]}]}"#;
        wb.deserialize(json).unwrap();
        let s = wb.sheet_id("Sheet1").unwrap();
        assert_eq!(
            wb.get_value(s, cell(1, 1)),
            Some(CellValue::Text("=THIS IS NOT A FORMULA".into()))
        );
    }

    // ── Range sort (Data ▸ Sort) ────────────────────────────────────

    fn rng(r0: u32, c0: u32, r1: u32, c1: u32) -> CellRange {
        CellRange::new(cell(r0, c0), cell(r1, c1))
    }

    #[test]
    fn sort_numbers_ascending_reorders_a_single_column() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, v) in [(1, 30.0), (2, 10.0), (3, 20.0)] {
            wb.set_value(s, cell(r, 1), CellValue::Number(v));
        }
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 1, true).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(10.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(20.0)));
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(30.0)));
    }

    #[test]
    fn sort_descending_reverses_the_order() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, v) in [(1, 1.0), (2, 3.0), (3, 2.0)] {
            wb.set_value(s, cell(r, 1), CellValue::Number(v));
        }
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 1, false).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(3.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(2.0)));
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(1.0)));
    }

    #[test]
    fn sort_carries_the_whole_record_and_its_format() {
        // Two columns: a key column (A) and a payload column (B) with a format.
        // Sorting by A must drag B's value AND format along with each row.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(2.0)); // A1 key
        wb.set_value(s, cell(2, 1), CellValue::Number(1.0)); // A2 key
        wb.set_value(s, cell(1, 2), CellValue::Number(200.0)); // B1 payload
        wb.set_value(s, cell(2, 2), CellValue::Number(100.0)); // B2 payload
        wb.set_format(s, cell(1, 2), "#,##0.00"); // format rides with B1's record

        assert!(wb.sort_range(s, rng(1, 1, 2, 2), 1, true).is_some());
        // Row that had key 1 (originally row 2) is now first.
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(100.0)));
        // The key-2 record moved to row 2, taking its B-column format with it.
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(2.0)));
        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Number(200.0)));
        assert_eq!(wb.get_format(s, cell(2, 2)), Some("#,##0.00"));
        assert_eq!(wb.get_format(s, cell(1, 2)), None); // the format moved away
    }

    #[test]
    fn sort_shifts_relative_refs_in_moved_formulas() {
        // B holds =A*10 in each row; sorting by A (so the rows move) must shift
        // each moved formula's relative ref by its row displacement, exactly like
        // a per-row cut/paste — every B still equals its own row's A times ten.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, v) in [(1, 3.0), (2, 1.0), (3, 2.0)] {
            wb.set_value(s, cell(r, 1), CellValue::Number(v));
            wb.set_formula(s, cell(r, 2), &format!("=A{r}*10")).unwrap();
        }
        assert!(wb.sort_range(s, rng(1, 1, 3, 2), 1, true).is_some());
        // Keys sorted to 1,2,3; each B is its row's A*10.
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Number(10.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(2.0)));
        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Number(20.0)));
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(3.0)));
        assert_eq!(wb.get_value(s, cell(3, 2)), Some(CellValue::Number(30.0)));
        // The formula at the new row 1 was shifted to point at A1.
        assert_eq!(wb.get_value(s, cell(1, 2)).unwrap(), CellValue::Number(10.0));
        let raw = match &wb.sheets[s.0 as usize].cells[&cell(1, 2)].content {
            CellContent::Formula { text, .. } => text.clone(),
            _ => panic!("expected a formula at B1"),
        };
        assert!(raw.contains("A1"), "moved formula should reference A1, got {raw}");
    }

    #[test]
    fn sort_is_stable_for_equal_keys() {
        // Two rows share key 1; a stable sort keeps their original B order.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(1, 2), CellValue::Text("first".into()));
        wb.set_value(s, cell(2, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(2, 2), CellValue::Text("second".into()));
        wb.set_value(s, cell(3, 1), CellValue::Number(0.0));
        wb.set_value(s, cell(3, 2), CellValue::Text("zero".into()));
        assert!(wb.sort_range(s, rng(1, 1, 3, 2), 1, true).is_some());
        // key 0 first, then the two key-1 rows in their original order.
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Text("zero".into())));
        assert_eq!(wb.get_value(s, cell(2, 2)), Some(CellValue::Text("first".into())));
        assert_eq!(wb.get_value(s, cell(3, 2)), Some(CellValue::Text("second".into())));
    }

    #[test]
    fn sort_text_is_case_insensitive() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, t) in [(1, "banana"), (2, "Apple"), (3, "cherry")] {
            wb.set_value(s, cell(r, 1), CellValue::Text(t.into()));
        }
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 1, true).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Text("Apple".into())));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Text("banana".into())));
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Text("cherry".into())));
    }

    #[test]
    fn sort_blanks_sink_last_in_both_directions() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(5.0));
        // row 2 left blank
        wb.set_value(s, cell(3, 1), CellValue::Number(1.0));
        // Ascending: 1, 5, blank.
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 1, true).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(5.0)));
        assert_eq!(wb.cell_value(s, cell(3, 1)), CellValue::Empty);
        // Descending: 5, 1, blank — the blank STILL sinks last.
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 1, false).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(5.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.cell_value(s, cell(3, 1)), CellValue::Empty);
    }

    #[test]
    fn sort_cross_type_order_number_before_text() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Text("zzz".into()));
        wb.set_value(s, cell(2, 1), CellValue::Number(99.0));
        assert!(wb.sort_range(s, rng(1, 1, 2, 1), 1, true).is_some());
        // Numbers (rank 0) sort before text (rank 1).
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(99.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Text("zzz".into())));
    }

    #[test]
    fn sort_leaves_cells_outside_the_column_band_untouched() {
        // The range is A:A; a value in B must not move when A is sorted.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(2.0));
        wb.set_value(s, cell(2, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(1, 2), CellValue::Text("stay".into())); // B1, outside range
        assert!(wb.sort_range(s, rng(1, 1, 2, 1), 1, true).is_some());
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(2.0)));
        // B1 did not move.
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Text("stay".into())));
        assert_eq!(wb.get_value(s, cell(2, 2)), None);
    }

    #[test]
    fn sort_rejects_bad_arguments() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        // key_col outside the range → false (no-op).
        assert!(wb.sort_range(s, rng(1, 1, 3, 1), 5, true).is_none());
        // Single-row range → false (nothing to reorder).
        assert!(wb.sort_range(s, rng(1, 1, 1, 1), 1, true).is_none());
        // Unknown sheet → false.
        assert!(wb.sort_range(SheetId(99), rng(1, 1, 3, 1), 1, true).is_none());
    }

    #[test]
    fn sort_already_sorted_is_a_noop_true_without_revision_bump() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(2, 1), CellValue::Number(2.0));
        let before = wb.current_revision();
        // Already ascending → returns true, but makes no change / no revision bump.
        assert!(wb.sort_range(s, rng(1, 1, 2, 1), 1, true).is_some());
        assert_eq!(wb.current_revision(), before);
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(2.0)));
    }

    #[test]
    fn sort_logs_moved_cells_for_changed_since() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(2.0));
        wb.set_value(s, cell(2, 1), CellValue::Number(1.0));
        let rev = wb.current_revision();
        assert!(wb.sort_range(s, rng(1, 1, 2, 1), 1, true).is_some());
        match wb.changed_since(s, rev) {
            ChangeSet::Delta { changed, .. } => {
                assert!(changed.contains(&cell(1, 1)), "A1 moved");
                assert!(changed.contains(&cell(2, 1)), "A2 moved");
            }
            ChangeSet::Stale { .. } => panic!("should be a Delta"),
        }
    }

    #[test]
    fn sort_returns_the_applied_permutation() {
        // Keys 30,10,20 → ascending order is the rows originally at offsets 1,2,0
        // (values 10,20,30). The returned permutation lets a caller replay the
        // exact move on its own side-table (the wasm facade's raw echo map).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (r, v) in [(1, 30.0), (2, 10.0), (3, 20.0)] {
            wb.set_value(s, cell(r, 1), CellValue::Number(v));
        }
        assert_eq!(wb.sort_range(s, rng(1, 1, 3, 1), 1, true), Some(vec![1, 2, 0]));
        // An already-sorted range reports the identity permutation (no change).
        assert_eq!(wb.sort_range(s, rng(1, 1, 3, 1), 1, true), Some(vec![0, 1, 2]));
        // A rejected sort reports None.
        assert_eq!(wb.sort_range(s, rng(1, 1, 3, 1), 9, true), None);
    }

    // ── Find / replace (Edit ▸ Find / Replace) ──────────────────────

    #[test]
    fn set_raw_routes_by_string_shape() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        // Number, boolean, text, formula, and empty (clear).
        wb.set_raw(s, cell(1, 1), "15");
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(15.0)));
        wb.set_raw(s, cell(1, 2), "TRUE");
        assert_eq!(wb.get_value(s, cell(1, 2)), Some(CellValue::Boolean(true)));
        wb.set_raw(s, cell(1, 3), "hello");
        assert_eq!(wb.get_value(s, cell(1, 3)), Some(CellValue::Text("hello".into())));
        wb.set_raw(s, cell(1, 4), "=A1*2"); // 30
        assert_eq!(wb.get_value(s, cell(1, 4)), Some(CellValue::Number(30.0)));
        wb.set_raw(s, cell(1, 1), "   "); // whitespace → clear
        assert_eq!(wb.get_value(s, cell(1, 1)), None);
        // A "=" string that won't parse degrades to a #VALUE! literal.
        wb.set_raw(s, cell(2, 1), "=this is not a formula(((");
        assert_eq!(
            wb.get_value(s, cell(2, 1)),
            Some(CellValue::Error(SpreadsheetError::Value))
        );
    }

    #[test]
    fn find_all_searches_values_or_sources_in_order() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_raw(s, cell(1, 1), "100");
        wb.set_raw(s, cell(3, 1), "hello world");
        wb.set_raw(s, cell(2, 2), "=A1+1"); // displays 101
        // Search computed VALUES: "10" is in 100 (A1) and 101 (B2's display).
        let by_value = wb.find_all(s, "10", false, true);
        assert_eq!(by_value, vec![cell(1, 1), cell(2, 2)]); // (row,col) order
        // Search SOURCES: "A1" appears only in B2's formula text, not in "100".
        assert_eq!(wb.find_all(s, "A1", true, true), vec![cell(2, 2)]);
        // Case-insensitive finds "HELLO" in "hello world".
        assert_eq!(wb.find_all(s, "HELLO", true, false), vec![cell(3, 1)]);
        assert!(wb.find_all(s, "HELLO", true, true).is_empty()); // case-sensitive misses
        // Empty query matches nothing.
        assert!(wb.find_all(s, "", false, true).is_empty());
    }

    #[test]
    fn replace_all_edits_sources_and_recomputes() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_raw(s, cell(1, 1), "10"); // A1
        wb.set_raw(s, cell(2, 1), "10"); // A2
        wb.set_raw(s, cell(3, 1), "=A1+A1"); // 20, formula referencing A1 twice
        // Replace the literal "10" → "7" in the two number cells (count = 2; the
        // formula's source "=A1+A1" has no "10").
        assert_eq!(wb.replace_all(s, "10", "7", true), 2);
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(7.0)));
        assert_eq!(wb.get_value(s, cell(2, 1)), Some(CellValue::Number(7.0)));
        // Now rewrite the formula's refs by replacing "A1" → "A2" in its source;
        // it re-parses and recomputes (=A2+A2 = 7+7 = 14).
        assert_eq!(wb.replace_all(s, "A1", "A2", true), 1);
        assert_eq!(wb.get_value(s, cell(3, 1)), Some(CellValue::Number(14.0)));
        // No match → 0 changes; empty query → 0.
        assert_eq!(wb.replace_all(s, "zzz", "q", true), 0);
        assert_eq!(wb.replace_all(s, "", "q", true), 0);
    }

    #[test]
    fn replace_all_is_case_insensitive_when_asked() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_raw(s, cell(1, 1), "Hello"); // text
        // Case-insensitive replace of "hello" → "Hi" splices over the original span.
        assert_eq!(wb.replace_all(s, "hello", "Hi", false), 1);
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Text("Hi".into())));
    }

    // ── Column widths & row heights ──────────────────────────────────────

    #[test]
    fn column_width_and_row_height_set_get_clear() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        // Absent → None (host uses its default).
        assert_eq!(wb.column_width(s, 3), None);
        assert_eq!(wb.row_height(s, 2), None);
        // Set, then read back.
        assert!(wb.set_column_width(s, 3, 140.0));
        assert!(wb.set_row_height(s, 2, 40.0));
        assert_eq!(wb.column_width(s, 3), Some(140.0));
        assert_eq!(wb.row_height(s, 2), Some(40.0));
        // Setting the SAME value is a no-op (returns false, revision unchanged).
        let rev = wb.current_revision();
        assert!(!wb.set_column_width(s, 3, 140.0));
        assert!(!wb.set_row_height(s, 2, 40.0));
        assert_eq!(wb.current_revision(), rev);
        // A different value overwrites + bumps the revision.
        assert!(wb.set_column_width(s, 3, 200.0));
        assert_eq!(wb.column_width(s, 3), Some(200.0));
        assert!(wb.current_revision() > rev);
        // Clear → back to None; clearing an absent one returns false.
        assert!(wb.clear_column_width(s, 3));
        assert_eq!(wb.column_width(s, 3), None);
        assert!(!wb.clear_column_width(s, 3));
        assert!(wb.clear_row_height(s, 2));
        assert_eq!(wb.row_height(s, 2), None);
    }

    #[test]
    fn set_size_rejects_bad_values_and_indices() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -5.0] {
            assert!(!wb.set_column_width(s, 1, bad), "width {bad} must be rejected");
            assert!(!wb.set_row_height(s, 1, bad), "height {bad} must be rejected");
        }
        // Index 0 is invalid (the grid is 1-based).
        assert!(!wb.set_column_width(s, 0, 100.0));
        assert!(!wb.set_row_height(s, 0, 100.0));
        // An unknown sheet is rejected, not panicked.
        assert!(!wb.set_column_width(SheetId(9), 1, 100.0));
        assert_eq!(wb.column_width(s, 1), None);
    }

    #[test]
    fn widths_and_heights_in_range_are_filtered_and_sorted() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        for (c, w) in [(2u32, 80.0), (5, 120.0), (3, 100.0), (9, 200.0)] {
            wb.set_column_width(s, c, w);
        }
        // Only columns in [3, 6], sorted ascending.
        assert_eq!(wb.column_widths_in(s, 3, 6), vec![(3, 100.0), (5, 120.0)]);
        // Row analogue.
        wb.set_row_height(s, 4, 30.0);
        wb.set_row_height(s, 1, 20.0);
        assert_eq!(wb.row_heights_in(s, 1, 4), vec![(1, 20.0), (4, 30.0)]);
        // Empty range / unknown sheet → empty.
        assert!(wb.column_widths_in(s, 100, 200).is_empty());
        assert!(wb.column_widths_in(SheetId(9), 1, 10).is_empty());
    }

    #[test]
    fn structural_edits_shift_width_and_height_keys() {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_column_width(s, 3, 140.0); // column C
        wb.set_row_height(s, 2, 40.0); // row 2

        // Insert a column at B (col 2): C's width slides to D (col 4). Heights untouched.
        wb.insert_cols(s, 2, 1);
        assert_eq!(wb.column_width(s, 3), None);
        assert_eq!(wb.column_width(s, 4), Some(140.0));
        assert_eq!(wb.row_height(s, 2), Some(40.0)); // other axis unmoved

        // Insert a row at 1: row 2's height slides to row 3. Widths untouched.
        wb.insert_rows(s, 1, 1);
        assert_eq!(wb.row_height(s, 2), None);
        assert_eq!(wb.row_height(s, 3), Some(40.0));
        assert_eq!(wb.column_width(s, 4), Some(140.0)); // width still at D

        // Delete the row holding the height (now row 3): its height is dropped.
        wb.delete_rows(s, 3, 1);
        assert_eq!(wb.row_height(s, 3), None);
        assert!(wb.row_heights_in(s, 1, 1000).is_empty());

        // Delete a column before the widened one (col 4 → back to col 3): width slides.
        wb.delete_cols(s, 1, 1);
        assert_eq!(wb.column_width(s, 3), Some(140.0));
        // Delete the widened column itself: its width is dropped.
        wb.delete_cols(s, 3, 1);
        assert_eq!(wb.column_width(s, 3), None);
        assert!(wb.column_widths_in(s, 1, 1000).is_empty());
    }

    #[test]
    fn range_sort_does_not_move_widths_or_heights() {
        // Resize is positional chrome, not record data: sorting the VALUES of rows
        // must leave the column widths and row heights where they are.
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(3.0));
        wb.set_value(s, cell(2, 1), CellValue::Number(1.0));
        wb.set_value(s, cell(3, 1), CellValue::Number(2.0));
        wb.set_column_width(s, 1, 150.0); // column A wide
        wb.set_row_height(s, 1, 50.0); // row 1 tall
        wb.sort_range(s, CellRange::new(cell(1, 1), cell(3, 1)), 1, true);
        // Values reordered (1,2,3) but A stays wide and row 1 stays tall.
        assert_eq!(wb.get_value(s, cell(1, 1)), Some(CellValue::Number(1.0)));
        assert_eq!(wb.column_width(s, 1), Some(150.0));
        assert_eq!(wb.row_height(s, 1), Some(50.0));
    }

    #[test]
    fn serialize_round_trips_widths_and_heights_across_two_sheets() {
        let mut wb = Workbook::new();
        let s1 = wb.add_sheet("Sheet1");
        let s2 = wb.add_sheet("Summary");
        wb.set_column_width(s1, 3, 140.0);
        wb.set_row_height(s1, 2, 40.0);
        wb.set_column_width(s2, 1, 90.5);
        let doc = wb.serialize();
        assert!(doc.contains("colWidths"));
        assert!(doc.contains("rowHeights"));

        let mut loaded = Workbook::new();
        loaded.deserialize(&doc).expect("round-trips");
        assert_eq!(loaded.column_width(SheetId(0), 3), Some(140.0));
        assert_eq!(loaded.row_height(SheetId(0), 2), Some(40.0));
        assert_eq!(loaded.column_width(SheetId(1), 1), Some(90.5));
    }

    #[test]
    fn deserialize_is_tolerant_of_missing_and_bad_sizes() {
        let mut wb = Workbook::new();
        // No size arrays at all (e.g. an old file) loads fine.
        wb.deserialize(r#"{"version":1,"sheets":[{"name":"S","cells":[],"formats":[]}]}"#)
            .expect("old-shape file loads");
        assert_eq!(wb.column_width(SheetId(0), 1), None);
        // A non-finite / ≤ 0 / index-0 entry is SKIPPED, not fatal.
        let doc = r#"{"version":1,"sheets":[{"name":"S","cells":[],"formats":[],
            "colWidths":[{"col":2,"w":120.0},{"col":3,"w":-9.0},{"col":0,"w":50.0}],
            "rowHeights":[{"row":1,"h":30.0}]}]}"#;
        let mut wb2 = Workbook::new();
        wb2.deserialize(doc).expect("tolerant load");
        assert_eq!(wb2.column_width(SheetId(0), 2), Some(120.0)); // good one took
        assert_eq!(wb2.column_width(SheetId(0), 3), None); // negative skipped
        assert_eq!(wb2.column_width(SheetId(0), 0), None); // index 0 skipped
        assert_eq!(wb2.row_height(SheetId(0), 1), Some(30.0));
    }

    #[test]
    fn serialize_of_no_custom_sizes_omits_the_arrays() {
        // A workbook with no resizes must serialize byte-identically to the
        // pre-feature output (no colWidths / rowHeights keys at all).
        let mut wb = Workbook::new();
        let s = wb.add_sheet("S");
        wb.set_value(s, cell(1, 1), CellValue::Number(15.0));
        let doc = wb.serialize();
        assert!(!doc.contains("colWidths"));
        assert!(!doc.contains("rowHeights"));
    }
}
