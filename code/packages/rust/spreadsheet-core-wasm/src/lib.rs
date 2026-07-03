//! # spreadsheet-core-wasm
//!
//! A **string-in / JSON-out facade** over [`spreadsheet_core`], shaped for a
//! browser/WASM embedding. The engine itself stays typed and Rust-native; this
//! crate draws a stable string boundary around it so a JavaScript host never
//! has to understand Rust types — it sends an A1 address and a raw cell string
//! in, and gets JSON back.
//!
//! It mirrors the repo's existing `macsyma-wasm` facade pattern: pure,
//! in-memory, panic-safe (a hardened engine *shouldn't* panic on adversarial
//! formulas, but we catch anyway so a stray panic degrades to an error value
//! instead of aborting the host / trapping the WASM module).
//!
//! ## Where it sits
//!
//! ```text
//!   JS host (VisiCalc demo)
//!        │  set_cell("B6", "=SUM(B1:B5)")   ── strings in
//!        ▼
//!   spreadsheet-core-wasm   ← this crate: A1 ⇄ CellAddress, raw ⇄ CellValue,
//!        │                     CellValue ⇄ JSON, panic-safety
//!        ▼
//!   spreadsheet-core        ← cells, dependency graph, recalc, formulas
//! ```
//!
//! This crate is the JSON facade. A thin `extern "C"` + linear-memory ABI and
//! the JS loader that instantiates the compiled `.wasm` are a separate layer
//! (so this crate stays a normal, `cargo test`-able workspace library — no
//! WASM toolchain needed to build or test it).
//!
//! ## The "raw" map
//!
//! A spreadsheet cell has two faces: what you *typed* (`=SUM(B1:B5)`) and what
//! it *shows* (`46`). The engine owns the computed value; this facade keeps a
//! small `raw` map of exactly what was set per cell, so the formula bar can be
//! repopulated with the source — the same split the TypeScript engine uses.
//!
//! ## Quick start
//!
//! ```rust
//! use spreadsheet_core_wasm::SpreadsheetSession;
//!
//! let mut s = SpreadsheetSession::new();
//! s.set_cell("B1", "15");
//! s.set_cell("B2", "8");
//! s.set_cell("B3", "=B1+B2");
//! assert_eq!(s.get_value("B3"), r#"{"kind":"number","value":23.0}"#);
//! assert_eq!(s.get_raw("B3"), "=B1+B2");
//! ```

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::{json, Value};
use spreadsheet_core::parser::parse;
use spreadsheet_core::address::MAX_RANGE_CELLS;
use spreadsheet_core::{
    column_index_to_letters, CellAddress, CellRange, CellValue, ChangeSet, SheetId,
    SpreadsheetError, StructuralEdit, Workbook,
};

/// A single-sheet spreadsheet session with a JSON boundary.
///
/// The original VisiCalc was single-sheet, and that is all the web demos need,
/// so this facade pins one sheet and addresses cells by bare A1 (`"B6"`). The
/// underlying [`Workbook`] is multi-sheet; a richer facade could expose that
/// later without changing this one.
pub struct SpreadsheetSession {
    wb: Workbook,
    /// The **active** sheet — the one bare-A1 reads/writes address. Switched by
    /// [`set_active_sheet`](Self::set_active_sheet); the sheet-management methods
    /// keep it valid across reorders/deletes.
    sheet: SheetId,
    /// What was literally typed into each cell of the **active** sheet — the
    /// source of truth for [`get_raw`](Self::get_raw), independent of the engine's
    /// internals. Inactive sheets' echoes live in [`other_raw`](Self::other_raw);
    /// switching the active sheet swaps the two.
    raw: HashMap<CellAddress, String>,
    /// Per-sheet raw echoes for every **non-active** sheet, keyed by `SheetId`.
    /// On [`set_active_sheet`](Self::set_active_sheet) the current `raw` is stashed
    /// here and the target sheet's map is taken out into `raw`. After a structural
    /// sheet op (add/rename/delete/move) that can reindex `SheetId`s, all echoes
    /// are rebuilt from the engine in one pass (`rebuild_all_raw_from_engine`).
    other_raw: HashMap<SheetId, HashMap<CellAddress, String>>,
    /// A facade-side mirror of the engine's clipboard, holding the *raw* (typed)
    /// source of each copied/cut cell so [`paste`](Self::paste) can keep the
    /// `raw` echo map in step — the engine stores parsed content, not the
    /// user's text. Its lifecycle tracks the engine's: kept on a copy, dropped
    /// on the paste that consumes a cut, untouched on a rejected paste.
    clip: Option<RawClip>,
    /// Undo history: serialized full-document snapshots of the state *before*
    /// each mutating edit, newest at the back. [`undo`](Self::undo) pops one and
    /// restores it; [`redo`](Self::redo) replays from `redo_stack`. We snapshot
    /// the whole document (source + formats, via [`serialize`](Self::serialize))
    /// rather than per-op inverses, so undo/redo is automatically correct for
    /// *every* edit — set, fill, clipboard, structural, format, load — and any
    /// future one, with no per-op bookkeeping. The clipboard buffer is a
    /// transient editing aid, not document state, so it is deliberately *not*
    /// captured (undo restores cells, not what's on the clipboard).
    undo_stack: Vec<String>,
    /// Redo history: snapshots of states undone away, newest at the back. Any new
    /// edit clears this (you can't redo past a fresh divergence) — the standard
    /// linear undo model.
    redo_stack: Vec<String>,
}

/// How many undo snapshots to retain. Old entries past this are dropped from the
/// front, so history is bounded regardless of session length (the oldest edits
/// become un-undoable, exactly like a real editor's finite history).
const MAX_HISTORY: usize = 100;

/// The facade's raw-text snapshot of a copied/cut rectangle, paired 1:1 with the
/// engine's clipboard. Offsets are from the source range's top-left anchor.
struct RawClip {
    anchor: CellAddress,
    source: CellRange,
    rows: u32,
    cols: u32,
    is_cut: bool,
    /// `(d_row, d_col) → raw text` for the non-blank source cells.
    cells: HashMap<(u32, u32), String>,
}

impl SpreadsheetSession {
    /// Create an empty session with one sheet.
    pub fn new() -> Self {
        let mut wb = Workbook::new();
        let sheet = wb.add_sheet("Sheet1");
        Self {
            wb,
            sheet,
            raw: HashMap::new(),
            other_raw: HashMap::new(),
            clip: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    // ── Multi-sheet workbook ─────────────────────────────────────────
    //
    // The facade addresses cells on one *active* sheet by bare A1; these methods
    // manage the set of sheets and which one is active. The underlying engine is
    // multi-sheet and cross-sheet-aware, so a formula on one sheet can reference
    // another (`=Summary!A1`) and recompute when that sheet changes — see the
    // engine's `spreadsheet-core`. Dense `SheetId`s are sheet indices, so a delete
    // or move reindexes them; after any such op the per-sheet raw echoes are
    // rebuilt from the engine in one pass so they can't go stale.

    /// The sheet names in tab order plus the active index, as JSON:
    /// `{"sheets":["Sheet1","Summary"],"active":0}`.
    pub fn sheet_names(&self) -> String {
        let names: Vec<&str> = self.wb.sheet_names();
        json!({ "sheets": names, "active": self.sheet.0 }).to_string()
    }

    /// The active sheet's 0-based index.
    pub fn active_sheet(&self) -> u32 {
        self.sheet.0
    }

    /// Switch the active sheet by 0-based index. The current sheet's raw echo is
    /// stashed and the target's taken out, so bare-A1 reads/writes now address the
    /// new sheet. Out-of-range index → no-op (`false`).
    pub fn set_active_sheet(&mut self, index: u32) -> bool {
        if index as usize >= self.wb.sheet_count() {
            return false;
        }
        self.activate(SheetId(index));
        true
    }

    /// Swap the active-sheet raw echo so `self.raw` is `target`'s map.
    fn activate(&mut self, target: SheetId) {
        if target == self.sheet {
            return;
        }
        let current = std::mem::take(&mut self.raw);
        self.other_raw.insert(self.sheet, current);
        self.raw = self.other_raw.remove(&target).unwrap_or_default();
        self.sheet = target;
    }

    /// Add a new sheet with `name` and make it active. Rejects a duplicate or
    /// empty name (`false`). Undoable.
    pub fn add_sheet(&mut self, name: &str) -> bool {
        if name.is_empty() || self.wb.sheet_id(name).is_some() {
            return false;
        }
        let name = name.to_string();
        self.mutate(|s| {
            let id = s.wb.add_sheet(name.clone());
            s.activate(id);
            true
        })
    }

    /// Rename the sheet at `index`. Rewrites the qualifier in every referencing
    /// formula (engine-side) and rebuilds the raw echoes so the formula bar shows
    /// the new name. Rejects empty/duplicate names or a bad index (`false`).
    pub fn rename_sheet(&mut self, index: u32, new_name: &str) -> bool {
        if index as usize >= self.wb.sheet_count() {
            return false;
        }
        let new_name = new_name.to_string();
        self.mutate(|s| {
            if s.wb.rename_sheet(SheetId(index), new_name.clone()).is_err() {
                return false;
            }
            s.rebuild_all_raw_from_engine();
            true
        })
    }

    /// Delete the sheet at `index`. Inbound references to it become `#REF!`; the
    /// remaining sheets are reindexed and the active sheet is kept valid (it
    /// follows its sheet, or falls to a neighbour if it was the one deleted).
    /// Refuses to delete the last sheet or a bad index (`false`). Undoable.
    pub fn delete_sheet(&mut self, index: u32) -> bool {
        let count = self.wb.sheet_count();
        if index as usize >= count || count <= 1 {
            return false;
        }
        let active_name = self.wb.sheet_name(self.sheet).map(str::to_string);
        self.mutate(|s| {
            if s.wb.delete_sheet(SheetId(index)).is_err() {
                return false;
            }
            // Re-resolve the active sheet: by name if it survived, else clamp to a
            // neighbour of the deleted position.
            // The active sheet survived → find it by name; otherwise (it was the
            // deleted one) clamp to the neighbour now at the deleted position.
            let last = s.wb.sheet_count() as u32 - 1;
            s.sheet = active_name
                .as_deref()
                .and_then(|n| s.wb.sheet_id(n))
                .unwrap_or(SheetId(index.min(last)));
            s.rebuild_all_raw_from_engine();
            true
        })
    }

    /// Move the sheet at `index` to 0-based `to_index` (clamped). The active sheet
    /// follows its tab. Bad index → no-op (`false`). Undoable.
    pub fn move_sheet(&mut self, index: u32, to_index: u32) -> bool {
        if index as usize >= self.wb.sheet_count() {
            return false;
        }
        let active_name = self.wb.sheet_name(self.sheet).map(str::to_string);
        self.mutate(|s| {
            if s.wb.move_sheet(SheetId(index), to_index as usize).is_err() {
                return false;
            }
            s.sheet = active_name
                .as_deref()
                .and_then(|n| s.wb.sheet_id(n))
                .unwrap_or(SheetId(0));
            s.rebuild_all_raw_from_engine();
            true
        })
    }

    /// Rebuild every sheet's raw echo from the engine's serialized document — used
    /// after a sheet op that can reindex `SheetId`s or rewrite qualifiers. The
    /// active sheet's map lands in `raw`, the rest in `other_raw`. Cell sources
    /// come from the engine's current text (re-emitted where the op rewrote a
    /// formula), the same normalization a structural edit already applies.
    fn rebuild_all_raw_from_engine(&mut self) {
        self.raw.clear();
        self.other_raw.clear();
        let doc = self.wb.serialize();
        let Ok(root) = serde_json::from_str::<Value>(&doc) else {
            return;
        };
        let Some(sheets) = root.get("sheets").and_then(Value::as_array) else {
            return;
        };
        for (i, sj) in sheets.iter().enumerate() {
            let mut map = HashMap::new();
            if let Some(cells) = sj.get("cells").and_then(Value::as_array) {
                for c in cells {
                    let Some(a1) = c.get("a1").and_then(Value::as_str) else {
                        continue;
                    };
                    let Ok(addr) = CellAddress::parse(a1) else {
                        continue;
                    };
                    let raw = if let Some(f) = c.get("formula").and_then(Value::as_str) {
                        f.to_string()
                    } else if let Some(vj) = c.get("value") {
                        raw_from_value_json(vj)
                    } else {
                        continue;
                    };
                    map.insert(addr, raw);
                }
            }
            let sid = SheetId(i as u32);
            if sid == self.sheet {
                self.raw = map;
            } else {
                self.other_raw.insert(sid, map);
            }
        }
    }

    // ── Undo / redo ─────────────────────────────────────────────────
    //
    // History is snapshot-based: every mutating edit is run through [`mutate`],
    // which captures the document (serialize) before the edit and, *only if the
    // edit actually changed something*, pushes that pre-state onto `undo_stack`
    // and clears `redo_stack`. So a no-op (a failed `set_cell`, a `copy` that
    // only touches the clipboard, an off-grid `fill`) leaves history untouched —
    // the user never has to press undo twice for one visible change. `undo`/
    // `redo` swap snapshots between the two stacks and restore via the same
    // machinery `deserialize` uses, so they correctly rebuild the formula-bar
    // echo and recompute every dependent.

    /// Run a mutating edit, recording an undo checkpoint iff it changed the
    /// document. `before`/`after` are full serializations; comparing them gates
    /// out no-ops so history stays meaningful (a sparse demo sheet serializes to
    /// a few hundred bytes, so the double-serialize is cheap).
    fn mutate<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let before = self.wb.serialize();
        let result = f(self);
        if self.wb.serialize() != before {
            self.undo_stack.push(before);
            // Bound the history: drop the oldest snapshot once we exceed the cap.
            if self.undo_stack.len() > MAX_HISTORY {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
        result
    }

    /// `true` if there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// `true` if there is an undone edit to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undo the most recent edit: restore the document to its state before that
    /// edit, pushing the current state onto the redo stack. Returns `false`
    /// (nothing happened) when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.wb.serialize());
        self.load_snapshot(&prev);
        true
    }

    /// Redo the most recently undone edit: restore the state that was undone
    /// away, pushing the current state back onto the undo stack. Returns `false`
    /// when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.wb.serialize());
        self.load_snapshot(&next);
        true
    }

    /// Set a cell from a raw, user-typed string and recompute dependents.
    ///
    /// The raw string is interpreted the way a spreadsheet does:
    /// - empty/whitespace → the cell is cleared;
    /// - a leading `=` → a formula;
    /// - `TRUE`/`FALSE` (any case) → a boolean;
    /// - anything that parses as a finite number → a number;
    /// - otherwise → text (a label).
    ///
    /// Returns a JSON object `{"ok": true}` on success, or
    /// `{"ok": false, "error": "..."}` if the address is malformed. A formula
    /// that fails to parse is *not* an error here — the cell stores `#VALUE!`
    /// and keeps the typed text (so the formula bar can still show it), exactly
    /// as a spreadsheet would.
    pub fn set_cell(&mut self, a1: &str, raw: &str) -> String {
        self.mutate(|s| {
            match catch_unwind(AssertUnwindSafe(|| s.set_cell_inner(a1, raw))) {
                Ok(Ok(())) => json!({ "ok": true }).to_string(),
                Ok(Err(msg)) => json!({ "ok": false, "error": msg }).to_string(),
                Err(_) => json!({ "ok": false, "error": "internal error" }).to_string(),
            }
        })
    }

    fn set_cell_inner(&mut self, a1: &str, raw: &str) -> Result<(), String> {
        let addr = CellAddress::parse(a1)
            .map_err(|e| format!("bad address '{a1}': {}", e.display()))?;
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            self.raw.remove(&addr);
            self.wb.set_value(self.sheet, addr, CellValue::Empty);
            return Ok(());
        }

        // Remember exactly what was typed, before any interpretation.
        self.raw.insert(addr, raw.to_string());

        if trimmed.starts_with('=') {
            if self.wb.set_formula(self.sheet, addr, trimmed).is_err() {
                // Unparseable formula → show #VALUE! but keep the source text.
                self.wb
                    .set_value(self.sheet, addr, CellValue::Error(SpreadsheetError::Value));
            }
        } else {
            self.wb.set_value(self.sheet, addr, coerce_literal(trimmed));
        }
        Ok(())
    }

    // ── Structural edits: insert / delete rows & columns ────────────
    //
    // These call through to the engine (which relocates cells and rewrites every
    // formula's references) AND keep this facade's `raw` echo map in step: each
    // raw entry's address is relocated the same way, and a formula's *source* is
    // rewritten via the shared `parse → adjust → to_formula_string` so the
    // formula bar echoes the post-edit references. Coordinates are 1-based.

    /// Insert `count` blank rows before row `at`; rows at/after slide down.
    pub fn insert_rows(&mut self, at: u32, count: u32) {
        self.structural_edit(StructuralEdit::InsertRows { at, count });
    }

    /// Delete `count` rows starting at row `at`; rows after slide up. Cells on
    /// deleted rows are removed; references to them become `#REF!`.
    pub fn delete_rows(&mut self, at: u32, count: u32) {
        self.structural_edit(StructuralEdit::DeleteRows { at, count });
    }

    /// Insert `count` blank columns before column `at`; columns at/after slide right.
    pub fn insert_cols(&mut self, at: u32, count: u32) {
        self.structural_edit(StructuralEdit::InsertCols { at, count });
    }

    /// Delete `count` columns starting at column `at`; columns after slide left.
    pub fn delete_cols(&mut self, at: u32, count: u32) {
        self.structural_edit(StructuralEdit::DeleteCols { at, count });
    }

    fn structural_edit(&mut self, edit: StructuralEdit) {
        self.mutate(|s| s.structural_edit_inner(edit));
    }

    fn structural_edit_inner(&mut self, edit: StructuralEdit) {
        // Mirror the engine's guard: an insert that would push a non-empty cell
        // off the u32 grid edge is rejected wholesale (the saturating shift would
        // otherwise collide raw entries onto the same address). Both sides apply
        // the same condition, so the facade and engine stay consistent.
        let would_overflow = match edit {
            StructuralEdit::InsertRows { at, count } => self
                .raw
                .keys()
                .any(|a| a.row >= at && a.row.checked_add(count).is_none()),
            StructuralEdit::InsertCols { at, count } => self
                .raw
                .keys()
                .any(|a| a.col >= at && a.col.checked_add(count).is_none()),
            StructuralEdit::DeleteRows { .. } | StructuralEdit::DeleteCols { .. } => false,
        };
        if would_overflow {
            return;
        }

        match edit {
            StructuralEdit::InsertRows { at, count } => self.wb.insert_rows(self.sheet, at, count),
            StructuralEdit::DeleteRows { at, count } => self.wb.delete_rows(self.sheet, at, count),
            StructuralEdit::InsertCols { at, count } => self.wb.insert_cols(self.sheet, at, count),
            StructuralEdit::DeleteCols { at, count } => self.wb.delete_cols(self.sheet, at, count),
        }

        // Relocate the raw echo map to match: move each entry's address, drop
        // entries on deleted lines, and rewrite formula sources.
        let old = std::mem::take(&mut self.raw);
        for (addr, raw) in old {
            if let Some(new_addr) = addr.adjust(edit) {
                self.raw.insert(new_addr, rewrite_raw_for_edit(&raw, edit));
            }
        }
    }

    /// Replicate the cell at `src_a1` across the inclusive rectangle
    /// `dst_start_a1`..`dst_end_a1` — drag-fill. Each target gets a copy with its
    /// formula's relative references shifted by its offset from the source
    /// (`=A1` filled down → `=A2`), absolute (`$`) references pinned, and the
    /// source's format carried along; an off-grid reference becomes `#REF!`. A
    /// literal source is copied unchanged; an empty source clears each target.
    /// Malformed addresses are a no-op (the engine and the echo map stay in
    /// step). Mirrors [`SpreadsheetSession::insert_rows`] in keeping the `raw`
    /// echo map honest — each target's stored source is the source's source with
    /// its references shifted (so the formula bar shows the filled formula).
    pub fn fill(&mut self, src_a1: &str, dst_start_a1: &str, dst_end_a1: &str) {
        self.mutate(|s| s.fill_inner(src_a1, dst_start_a1, dst_end_a1));
    }

    fn fill_inner(&mut self, src_a1: &str, dst_start_a1: &str, dst_end_a1: &str) {
        let (Ok(src), Ok(ds), Ok(de)) = (
            CellAddress::parse(src_a1),
            CellAddress::parse(dst_start_a1),
            CellAddress::parse(dst_end_a1),
        ) else {
            return;
        };
        let dst = CellRange::new(ds, de);
        // Mirror the engine's DoS guard so the raw-map loop below also stays
        // bounded — without it a hostile `dst` could make the facade iterate
        // billions of cells even though the engine itself rejected the fill.
        if dst.cell_count() > MAX_RANGE_CELLS {
            return;
        }

        // The engine replicates cell content + formats (shifting formula refs).
        self.wb.fill(self.sheet, src, dst);

        // Keep the `raw` echo map in step: each target's source is the source
        // cell's raw text with its references shifted by the target's offset
        // (formulas rewritten via parse→shift→serialize; literals copied; an
        // empty source clears the target). Offsets in i64 then clamped, matching
        // the engine — a high-coordinate anchor can't overflow the i32 delta.
        let src_raw = self.raw.get(&src).cloned();
        for row in dst.start.row..=dst.end.row {
            for col in dst.start.col..=dst.end.col {
                let target = CellAddress::new(row, col);
                let d_row =
                    (row as i64 - src.row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                let d_col =
                    (col as i64 - src.col as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                match &src_raw {
                    Some(raw) => {
                        self.raw.insert(target, rewrite_raw_for_fill(raw, d_row, d_col));
                    }
                    None => {
                        self.raw.remove(&target);
                    }
                }
            }
        }
    }

    /// Sort the rows of the inclusive rectangle `start_a1`..`end_a1` by the
    /// computed values in `key_col` (a 1-based absolute column index that must lie
    /// inside the rectangle) — *Data ▸ Sort*. Each row moves as a record; the sort
    /// key is the cell's computed value at the key column (a formula sorts by its
    /// result), under a fixed total order (blanks last both directions; Number <
    /// Text < Boolean < Error; case-insensitive text; stable). `ascending = false`
    /// reverses only the non-empty comparison. Moved formulas have their relative
    /// references shifted by the row displacement (absolute `$` refs pinned,
    /// off-grid → `#REF!`), and formats ride along — exactly as the engine does.
    ///
    /// Returns `true` once a valid sort is applied (or the range was already
    /// sorted), `false` for a malformed address, an out-of-range `key_col`, an
    /// empty/single-row range, or a rectangle over `MAX_RANGE_CELLS`. Mirrors
    /// [`fill`](Self::fill) in keeping the `raw` echo map honest: it replays the
    /// engine's permutation onto the stored sources so the formula bar shows each
    /// moved cell's (reference-shifted) source.
    pub fn sort_range(&mut self, start_a1: &str, end_a1: &str, key_col: u32, ascending: bool) -> bool {
        self.mutate(|s| s.sort_range_inner(start_a1, end_a1, key_col, ascending))
    }

    fn sort_range_inner(&mut self, start_a1: &str, end_a1: &str, key_col: u32, ascending: bool) -> bool {
        let (Ok(start), Ok(end)) = (CellAddress::parse(start_a1), CellAddress::parse(end_a1)) else {
            return false;
        };
        let range = CellRange::new(start, end);
        // Mirror the engine's DoS guard so the raw-map replay below stays bounded.
        if range.cell_count() > MAX_RANGE_CELLS {
            return false;
        }

        // The engine permutes cell content + formats and hands back the row
        // permutation it applied (`order[new_offset] = old_offset`); `None` is a
        // rejected/empty sort. We replay that exact permutation onto the `raw`
        // echo map so the formula bar stays in step.
        let Some(order) = self.wb.sort_range(self.sheet, range, key_col, ascending) else {
            return false;
        };

        // Snapshot the range's raw sources before rewriting (a permutation
        // overwrites entries in place), keyed by (row offset, col).
        let first = range.start.row;
        let mut snap: HashMap<(u32, u32), String> = HashMap::new();
        for (i, _) in order.iter().enumerate() {
            let row = first + i as u32;
            for col in range.start.col..=range.end.col {
                if let Some(raw) = self.raw.get(&CellAddress::new(row, col)) {
                    snap.insert((i as u32, col), raw.clone());
                }
            }
        }
        // Rewrite each destination row from its source row, shifting formula
        // sources by the row displacement (Δcol = 0) — the same arithmetic the
        // engine applied to the cells, via `rewrite_raw_for_fill`.
        for (new_i, &old_i) in order.iter().enumerate() {
            let dest_row = first + new_i as u32;
            let src_row = first + old_i;
            let d_row =
                (dest_row as i64 - src_row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            for col in range.start.col..=range.end.col {
                let dest = CellAddress::new(dest_row, col);
                match snap.get(&(old_i, col)) {
                    Some(raw) => {
                        self.raw.insert(dest, rewrite_raw_for_fill(raw, d_row, 0));
                    }
                    None => {
                        self.raw.remove(&dest);
                    }
                }
            }
        }
        true
    }

    /// Find every cell whose text contains `query` — the engine's `find_all`
    /// behind a JSON return. `in_formulas` searches the cell's **source** (formula
    /// text / literal canonical string) when true, its **computed display** value
    /// when false; `match_case = false` folds ASCII case. Returns
    /// `{"matches":["A1","C3",…]}` (the A1 addresses, in (row,col) order); an empty
    /// query yields an empty list. Read-only — no `mutate`, no `raw` change.
    pub fn find_all(&self, query: &str, in_formulas: bool, match_case: bool) -> String {
        let matches: Vec<Value> = self
            .wb
            .find_all(self.sheet, query, in_formulas, match_case)
            .iter()
            .map(|a| Value::String(a.to_a1()))
            .collect();
        json!({ "matches": matches }).to_string()
    }

    /// Replace `query` with `replacement` in the **source** of every matching
    /// cell, via the engine's `replace_all` (which rewrites + recomputes), then
    /// resync this facade's `raw` echo for the changed cells from the engine's new
    /// source text so the formula bar stays in step. Returns the count of cells
    /// changed; an empty query is a no-op. Routed through `mutate` for undo/redo.
    pub fn replace_all(&mut self, query: &str, replacement: &str, match_case: bool) -> u32 {
        self.mutate(|s| s.replace_all_inner(query, replacement, match_case))
    }

    fn replace_all_inner(&mut self, query: &str, replacement: &str, match_case: bool) -> u32 {
        if query.is_empty() {
            return 0;
        }
        // The cells replace_all will edit are exactly those whose SOURCE matches
        // (in_formulas = true). Capture them up front so we can resync `raw` after.
        let hits = self.wb.find_all(self.sheet, query, true, match_case);
        let count = self.wb.replace_all(self.sheet, query, replacement, match_case) as u32;
        // Resync the raw echo from the engine's post-replace source text: a cell
        // cleared to empty drops out of the map; otherwise it holds the new source.
        for addr in &hits {
            let src = self.wb.cell_source_text(self.sheet, *addr);
            if src.is_empty() {
                self.raw.remove(addr);
            } else {
                self.raw.insert(*addr, src);
            }
        }
        count
    }

    /// Copy the inclusive rectangle `start_a1`..`end_a1` into the clipboard — a
    /// whole-block copy that pastes as a unit (the sibling of [`fill`](Self::fill),
    /// which replicates one cell). Content + format are captured by the engine;
    /// this facade also snapshots each cell's raw source so [`paste`](Self::paste)
    /// can keep the formula-bar echo in step. The source is left untouched; the
    /// buffer survives any number of pastes. Malformed addresses or a rectangle
    /// over `MAX_RANGE_CELLS` are a no-op.
    pub fn copy(&mut self, start_a1: &str, end_a1: &str) {
        self.snapshot(start_a1, end_a1, false);
    }

    /// Cut the inclusive rectangle `start_a1`..`end_a1` into the clipboard. Like
    /// [`copy`](Self::copy) but a one-shot move: the [`paste`](Self::paste) that
    /// places it clears the source cells it didn't overwrite and consumes the
    /// buffer. The source is not cleared until paste.
    pub fn cut(&mut self, start_a1: &str, end_a1: &str) {
        self.snapshot(start_a1, end_a1, true);
    }

    /// Shared capture for [`copy`]/[`cut`]: drive the engine's clipboard and
    /// mirror the raw echo into a [`RawClip`].
    fn snapshot(&mut self, start_a1: &str, end_a1: &str, is_cut: bool) {
        let (Ok(start), Ok(end)) = (CellAddress::parse(start_a1), CellAddress::parse(end_a1)) else {
            return;
        };
        let range = CellRange::new(start, end);
        // Mirror the engine's DoS guard so the raw-snapshot loop stays bounded.
        if range.cell_count() > MAX_RANGE_CELLS {
            return;
        }

        if is_cut {
            self.wb.cut(self.sheet, range);
        } else {
            self.wb.copy(self.sheet, range);
        }

        let anchor = range.start;
        let mut cells = HashMap::new();
        for row in range.start.row..=range.end.row {
            for col in range.start.col..=range.end.col {
                if let Some(raw) = self.raw.get(&CellAddress::new(row, col)) {
                    cells.insert((row - anchor.row, col - anchor.col), raw.clone());
                }
            }
        }
        self.clip = Some(RawClip {
            anchor,
            source: range,
            rows: range.end.row - range.start.row + 1,
            cols: range.end.col - range.start.col + 1,
            is_cut,
            cells,
        });
    }

    /// Paste the clipboard so its top-left lands at `dst_start_a1`. Returns `true`
    /// when applied, `false` (a no-op) for an empty clipboard, a malformed
    /// address, or a destination that would run past the grid edge — exactly
    /// tracking the engine's `paste`. The whole block's references shift by the
    /// destination's offset from the source anchor; content, format, and the raw
    /// echo all ride along, and source blanks erase their targets. A cut then
    /// clears the source echo it didn't overwrite and consumes the buffer.
    pub fn paste(&mut self, dst_start_a1: &str) -> bool {
        self.mutate(|s| s.paste_inner(dst_start_a1))
    }

    fn paste_inner(&mut self, dst_start_a1: &str) -> bool {
        let Some(clip) = self.clip.take() else {
            return false;
        };
        let Ok(dst) = CellAddress::parse(dst_start_a1) else {
            self.clip = Some(clip); // bad address — keep the buffer
            return false;
        };

        // Drive the engine; it enforces every guard (off-grid, sheet, bounds) and
        // reports whether a paste happened. If it declined, leave the raw echo
        // and the facade buffer exactly as they were.
        if !self.wb.paste(self.sheet, dst) {
            self.clip = Some(clip);
            return false;
        }

        // Engine pasted: rewrite the raw echo for the destination rectangle. The
        // whole block shifts by the same delta (dst − anchor), i64-clamped into
        // the i32 contract like fill, so a high-coordinate paste can't overflow.
        let d_row =
            (dst.row as i64 - clip.anchor.row as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let d_col =
            (dst.col as i64 - clip.anchor.col as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        for dr in 0..clip.rows {
            for dc in 0..clip.cols {
                let target = CellAddress::new(dst.row + dr, dst.col + dc);
                match clip.cells.get(&(dr, dc)) {
                    Some(raw) => {
                        self.raw.insert(target, rewrite_raw_for_fill(raw, d_row, d_col));
                    }
                    None => {
                        self.raw.remove(&target); // source blank → erase target echo
                    }
                }
            }
        }

        // A cut moves: clear the source echo the paste didn't overwrite.
        if clip.is_cut {
            let dst_end_row = dst.row + clip.rows - 1;
            let dst_end_col = dst.col + clip.cols - 1;
            for row in clip.source.start.row..=clip.source.end.row {
                for col in clip.source.start.col..=clip.source.end.col {
                    let covered = row >= dst.row
                        && row <= dst_end_row
                        && col >= dst.col
                        && col <= dst_end_col;
                    if !covered {
                        self.raw.remove(&CellAddress::new(row, col));
                    }
                }
            }
        }

        // Copy's buffer survives for reuse; a cut's is consumed (matching engine).
        if !clip.is_cut {
            self.clip = Some(clip);
        }
        true
    }

    /// Whether the clipboard currently holds a copied/cut block.
    pub fn has_clipboard(&self) -> bool {
        self.clip.is_some()
    }

    /// Serialize the workbook to a portable JSON string (save). Delegates to the
    /// engine's [`Workbook::serialize`], which captures every source cell + format
    /// per sheet; the JSON round-trips through [`deserialize`](Self::deserialize).
    /// No I/O — the JS host stores the returned string wherever it likes.
    pub fn serialize(&self) -> String {
        self.wb.serialize()
    }

    /// Replace the workbook's contents from a JSON string produced by
    /// [`serialize`](Self::serialize) (load). Returns `true` on success, `false`
    /// for malformed JSON / unsupported version / bad structure (the engine
    /// leaves itself untouched on a bad header). On success the facade's `raw`
    /// echo map is rebuilt from the loaded JSON so the formula bar shows each
    /// cell's source (a formula's text; a literal's canonical string), and the
    /// pinned single sheet is re-bound (a zero-sheet file gets a fresh "Sheet1"
    /// so the facade stays usable).
    pub fn deserialize(&mut self, data: &str) -> bool {
        // A file-load is an undoable edit, so route it through `mutate` (which
        // checkpoints iff the document actually changed). undo/redo restore via
        // `load_snapshot` directly, bypassing history.
        self.mutate(|s| s.load_snapshot(data))
    }

    /// Restore the document from a JSON snapshot, *without* touching history.
    /// Shared by the public [`deserialize`](Self::deserialize) (a user load) and
    /// by [`undo`](Self::undo)/[`redo`](Self::redo). Returns `true` on success,
    /// `false` for malformed JSON / unsupported version / bad structure (the
    /// engine leaves itself untouched on a bad header).
    fn load_snapshot(&mut self, data: &str) -> bool {
        if self.wb.deserialize(data).is_err() {
            return false;
        }
        // Re-bind the active sheet to the first loaded sheet (file order →
        // SheetId(0)), or a fresh "Sheet1" if the file had none, rather than
        // leaving it pointing at a hole.
        self.sheet = if self.wb.sheet_count() == 0 {
            self.wb.add_sheet("Sheet1")
        } else {
            SheetId(0)
        };
        // Rebuild every sheet's raw echo from the engine (the engine doesn't keep
        // the user's typed text). Resets `other_raw` too, so a load of a different
        // sheet count can't leave stale inactive echoes behind.
        self.rebuild_all_raw_from_engine();
        true
    }

    /// Get a cell's *computed* value as a JSON object (see [`value_to_json`] for
    /// the shape). A malformed address yields an `#REF!`-style error object
    /// rather than failing.
    pub fn get_value(&self, a1: &str) -> String {
        match CellAddress::parse(a1) {
            Ok(addr) => {
                let v = self.wb.get_value(self.sheet, addr).unwrap_or(CellValue::Empty);
                value_to_json(&v).to_string()
            }
            Err(e) => json!({ "kind": "error", "code": e.display() }).to_string(),
        }
    }

    /// Get the raw, user-typed source of a cell (a formula or literal), or the
    /// empty string if the cell was never set. This is what the formula bar
    /// shows when a cell is selected.
    pub fn get_raw(&self, a1: &str) -> String {
        CellAddress::parse(a1)
            .ok()
            .and_then(|addr| self.raw.get(&addr).cloned())
            .unwrap_or_default()
    }

    // ── Cell display formats ────────────────────────────────────────
    //
    // A format is an Excel-style code (`"#,##0.00"`, `"0%"`, `"yyyy-mm-dd"`) that
    // decides how a cell's computed value reads. The engine stores the code and
    // applies it (via number-format-core); these thin wrappers expose that to a
    // JS / native host.

    /// Set a cell's display format code. An empty code clears it (the cell falls
    /// back to `General`). A malformed address is a no-op.
    pub fn set_format(&mut self, a1: &str, code: &str) {
        self.mutate(|s| {
            if let Ok(addr) = CellAddress::parse(a1) {
                let sheet = s.sheet;
                s.wb.set_format(sheet, addr, code);
            }
        });
    }

    /// A cell's display format code, or `""` if it uses the default (`General`).
    pub fn get_format(&self, a1: &str) -> String {
        CellAddress::parse(a1)
            .ok()
            .and_then(|addr| self.wb.get_format(self.sheet, addr))
            .map(str::to_string)
            .unwrap_or_default()
    }

    /// A cell's computed value rendered through its format — the **display
    /// string** to show (e.g. `1234.5` with `"#,##0.00"` → `"1,234.50"`). What a
    /// cell paints, as opposed to [`get_value`](Self::get_value) (typed JSON) or
    /// [`get_raw`](Self::get_raw) (the source). Empty string for a bad address.
    pub fn get_display(&self, a1: &str) -> String {
        match CellAddress::parse(a1) {
            Ok(addr) => self.wb.get_display(self.sheet, addr),
            Err(_) => String::new(),
        }
    }

    // ── Column widths & row heights ──────────────────────────────────
    //
    // Per-column / per-row sizes on the ACTIVE sheet (bare-index, like every
    // other session op). The engine stores an opaque size keyed by column / row;
    // a host renders columns / rows at these sizes and uses its own default where
    // none is set. `0.0` is the unset sentinel here — a valid size is always
    // `> 0`, so a host treats `0.0` as "use my default". Resizes are undoable
    // (routed through `mutate`) and persist through save / load.

    /// The width of a 1-based `col` on the active sheet, or `0.0` if the column
    /// has no custom width (the host should use its default).
    pub fn column_width(&self, col: u32) -> f64 {
        self.wb.column_width(self.sheet, col).unwrap_or(0.0)
    }

    /// The height of a 1-based `row` on the active sheet, or `0.0` if unset.
    pub fn row_height(&self, row: u32) -> f64 {
        self.wb.row_height(self.sheet, row).unwrap_or(0.0)
    }

    /// Set the width of a 1-based `col` on the active sheet. Returns `true` if it
    /// changed. A non-finite / `≤ 0` width or `col == 0` is rejected (`false`) by
    /// the engine, so a bad host value can't poison the sheet. Undoable.
    pub fn set_column_width(&mut self, col: u32, width: f64) -> bool {
        self.mutate(|s| {
            let sheet = s.sheet;
            s.wb.set_column_width(sheet, col, width)
        })
    }

    /// Set the height of a 1-based `row` on the active sheet. The row analogue of
    /// [`set_column_width`](Self::set_column_width). Undoable.
    pub fn set_row_height(&mut self, row: u32, height: f64) -> bool {
        self.mutate(|s| {
            let sheet = s.sheet;
            s.wb.set_row_height(sheet, row, height)
        })
    }

    /// Clear a column's custom width on the active sheet (back to the host
    /// default). Returns `true` if a width was removed. Undoable.
    pub fn clear_column_width(&mut self, col: u32) -> bool {
        self.mutate(|s| {
            let sheet = s.sheet;
            s.wb.clear_column_width(sheet, col)
        })
    }

    /// Clear a row's custom height on the active sheet. Undoable.
    pub fn clear_row_height(&mut self, row: u32) -> bool {
        self.mutate(|s| {
            let sheet = s.sheet;
            s.wb.clear_row_height(sheet, row)
        })
    }

    /// Every customized column width in the inclusive 1-based range `[col0, col1]`
    /// on the active sheet, as JSON `[{"col":3,"w":140.0},...]` sorted by column —
    /// a host fetches a viewport's overrides in one call.
    pub fn column_widths(&self, col0: u32, col1: u32) -> String {
        let ws: Vec<serde_json::Value> = self
            .wb
            .column_widths_in(self.sheet, col0, col1)
            .into_iter()
            .map(|(c, w)| json!({ "col": c, "w": w }))
            .collect();
        serde_json::Value::Array(ws).to_string()
    }

    /// Every customized row height in the inclusive 1-based range `[row0, row1]`
    /// on the active sheet, as JSON `[{"row":2,"h":40.0},...]` sorted by row.
    pub fn row_heights(&self, row0: u32, row1: u32) -> String {
        let hs: Vec<serde_json::Value> = self
            .wb
            .row_heights_in(self.sheet, row0, row1)
            .into_iter()
            .map(|(r, h)| json!({ "row": r, "h": h }))
            .collect();
        serde_json::Value::Array(hs).to_string()
    }

    /// Get every set cell's computed value as a JSON object keyed by A1
    /// address, e.g. `{"B1":{"kind":"number","value":15.0},...}`. Only cells
    /// that were explicitly set appear; everything else is implicitly empty.
    pub fn get_values(&self) -> String {
        let mut map = serde_json::Map::new();
        for addr in self.raw.keys() {
            let v = self.wb.get_value(self.sheet, *addr).unwrap_or(CellValue::Empty);
            map.insert(addr.to_a1(), value_to_json(&v));
        }
        Value::Object(map).to_string()
    }

    // ── Viewport primitive (virtualized infinite sheet) ──────────────
    //
    // These mirror the engine's `Workbook::get_window` / `used_range` /
    // `changed_since` reads (1-based, inclusive coords) so a JS host can render
    // only the visible window of an unbounded sheet. Coordinates are integers
    // here, not A1 strings — a scrolling host computes them from pixel offsets.

    /// Computed values for the inclusive 1-based rectangle, as JSON:
    /// `{"row0":1,"col0":1,"rows":R,"cols":C,"values":[[<value>,…],…]}` where
    /// each `<value>` is the usual value-object shape and `values` is a row-major
    /// `R×C` array (empty cells included as `{"kind":"empty"}`). On a bad
    /// request (inverted/oversized/0-coord) returns `{"error":"#REF!"}`.
    pub fn get_window(&self, row0: u32, col0: u32, row1: u32, col1: u32) -> String {
        match self.wb.get_window(self.sheet, row0, col0, row1, col1) {
            Ok(w) => {
                let mut rows = Vec::with_capacity(w.rows as usize);
                for r in 0..w.rows {
                    let mut row = Vec::with_capacity(w.cols as usize);
                    for c in 0..w.cols {
                        row.push(value_to_json(&w.values[(r * w.cols + c) as usize]));
                    }
                    rows.push(Value::Array(row));
                }
                json!({
                    "row0": w.row0, "col0": w.col0,
                    "rows": w.rows, "cols": w.cols,
                    "values": rows,
                })
                .to_string()
            }
            Err(e) => json!({ "error": e.display() }).to_string(),
        }
    }

    /// Display **strings** for the inclusive 1-based rectangle, as JSON:
    /// `{"row0":1,"col0":1,"rows":R,"cols":C,"cells":[["1,234.50",…],…]}` where
    /// `cells` is a row-major `R×C` array of the per-cell display strings (each
    /// value already rendered through its format code; empty cells are `""`).
    /// This is the format-aware sibling of [`get_window`](Self::get_window) — the
    /// one read a virtualized grid needs per frame, since the host paints the
    /// strings directly without re-deriving number formatting. On a bad request
    /// (inverted/oversized/0-coord) returns `{"error":"#REF!"}`.
    pub fn get_display_window(&self, row0: u32, col0: u32, row1: u32, col1: u32) -> String {
        match self.wb.get_display_window(self.sheet, row0, col0, row1, col1) {
            Ok(w) => {
                let mut rows = Vec::with_capacity(w.rows as usize);
                for r in 0..w.rows {
                    let mut row = Vec::with_capacity(w.cols as usize);
                    for c in 0..w.cols {
                        row.push(Value::String(w.cells[(r * w.cols + c) as usize].clone()));
                    }
                    rows.push(Value::Array(row));
                }
                json!({
                    "row0": w.row0, "col0": w.col0,
                    "rows": w.rows, "cols": w.cols,
                    "cells": rows,
                })
                .to_string()
            }
            Err(e) => json!({ "error": e.display() }).to_string(),
        }
    }

    /// The data extent as JSON `{"minRow":…,"minCol":…,"maxRow":…,"maxCol":…}`,
    /// or the JSON literal `null` if the sheet has no non-empty cells. A host
    /// sizes its scrollable area to this.
    pub fn used_range(&self) -> String {
        match self.wb.used_range(self.sheet) {
            Some(u) => json!({
                "minRow": u.min_row, "minCol": u.min_col,
                "maxRow": u.max_row, "maxCol": u.max_col,
            })
            .to_string(),
            None => "null".to_string(),
        }
    }

    /// The column letters for a 1-based column index (`1` → `"A"`, `27` → `"AA"`).
    /// Hosts use this for the frozen header row instead of re-implementing the
    /// base-26-bijective math.
    pub fn column_letters(&self, index: u32) -> String {
        column_index_to_letters(index)
    }

    /// The per-edit revision clock. A host snapshots this, then passes it to
    /// [`changed_since`](Self::changed_since) to learn what changed in between.
    pub fn current_revision(&self) -> u64 {
        self.wb.current_revision()
    }

    /// Which cells changed since `since_revision`, as JSON:
    /// `{"revision":N,"changed":["B2",…]}` (a complete deduped list), or
    /// `{"revision":N,"stale":true}` when the query reaches before the retained
    /// change log — the host then re-reads its whole visible window.
    pub fn changed_since(&self, since_revision: u64) -> String {
        match self.wb.changed_since(self.sheet, since_revision) {
            ChangeSet::Delta { current_revision, changed } => {
                let cells: Vec<Value> =
                    changed.iter().map(|a| Value::String(a.to_a1())).collect();
                json!({ "revision": current_revision, "changed": cells }).to_string()
            }
            ChangeSet::Stale { current_revision } => {
                json!({ "revision": current_revision, "stale": true }).to_string()
            }
        }
    }
}

impl Default for SpreadsheetSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpret a non-formula raw string as a literal cell value, the way a
/// spreadsheet does: booleans first, then a finite number, else text.
fn coerce_literal(s: &str) -> CellValue {
    if s.eq_ignore_ascii_case("true") {
        return CellValue::Boolean(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return CellValue::Boolean(false);
    }
    if let Ok(n) = s.parse::<f64>() {
        if n.is_finite() {
            return CellValue::Number(n);
        }
    }
    CellValue::Text(s.to_string())
}

/// Rewrite a raw cell source for a [`StructuralEdit`], so the formula bar echoes
/// the post-edit references. A literal is unchanged (it has no references); a
/// formula is re-parsed, its references adjusted via the shared `edit` arithmetic,
/// and re-serialized. An unparseable formula is kept verbatim — there's nothing
/// to rewrite, and the source must survive for the user to fix.
fn rewrite_raw_for_edit(raw: &str, edit: StructuralEdit) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('=') {
        match parse(trimmed) {
            Ok(ast) => format!("={}", ast.adjust(edit).to_formula_string()),
            Err(_) => raw.to_string(),
        }
    } else {
        raw.to_string()
    }
}

/// Rewrite a raw cell source for a fill of `(d_row, d_col)`, so the formula bar
/// echoes the *shifted* references the engine stored — the copy/paste sibling of
/// [`rewrite_raw_for_edit`]. A literal is copied verbatim (no references); a
/// formula is re-parsed, its references shifted via the shared `shift` arithmetic
/// (relative tracks, absolute pinned, off-grid → `#REF!`), and re-serialized. An
/// unparseable formula is kept as-is — there's nothing to rewrite.
fn rewrite_raw_for_fill(raw: &str, d_row: i32, d_col: i32) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('=') {
        match parse(trimmed) {
            Ok(ast) => format!("={}", ast.shift(d_row, d_col).to_formula_string()),
            Err(_) => raw.to_string(),
        }
    } else {
        raw.to_string()
    }
}

/// Reconstruct a literal cell's `raw` echo from the engine's serialized `value`
/// object (`{"number":n}` / `{"text":s}` / `{"bool":b}` / `{"error":code}`) — the
/// string a user would type to re-enter it. Used by [`SpreadsheetSession::deserialize`]
/// to repopulate the formula-bar source after a load (the engine stores typed
/// values, not the original text). Integers render without a trailing `.0`,
/// matching how the grid shows them.
fn raw_from_value_json(vj: &Value) -> String {
    if let Some(b) = vj.get("bool").and_then(Value::as_bool) {
        if b { "TRUE".to_string() } else { "FALSE".to_string() }
    } else if let Some(n) = vj.get("number").and_then(Value::as_f64) {
        if n == n.trunc() && n.abs() < 1e15 {
            (n as i64).to_string()
        } else {
            n.to_string()
        }
    } else if let Some(t) = vj.get("text").and_then(Value::as_str) {
        t.to_string()
    } else if let Some(code) = vj.get("error").and_then(Value::as_str) {
        code.to_string()
    } else {
        String::new()
    }
}

/// Encode a [`CellValue`] as the JSON the JS host expects. The shape matches
/// the TypeScript engine's `CellValue` discriminated union exactly, so the
/// demo glue is identical whichever engine backs it:
///
/// | value        | JSON                                      |
/// |--------------|-------------------------------------------|
/// | `Empty`      | `{"kind":"empty"}`                        |
/// | `Number(n)`  | `{"kind":"number","value":n}`             |
/// | `Text(s)`    | `{"kind":"text","value":"s"}`             |
/// | `Boolean(b)` | `{"kind":"boolean","value":b}`            |
/// | `Error(e)`   | `{"code":"#DIV/0!","kind":"error"}`       |
///
/// A non-finite number (NaN / ±∞, which JSON cannot represent) is reported as
/// `#NUM!` rather than being silently serialized to `null`.
fn value_to_json(v: &CellValue) -> Value {
    match v {
        CellValue::Empty => json!({ "kind": "empty" }),
        CellValue::Boolean(b) => json!({ "kind": "boolean", "value": b }),
        CellValue::Number(n) if n.is_finite() => json!({ "kind": "number", "value": n }),
        CellValue::Number(_) => {
            json!({ "kind": "error", "code": SpreadsheetError::Num.display() })
        }
        CellValue::Text(s) => json!({ "kind": "text", "value": s }),
        CellValue::Error(e) => json!({ "kind": "error", "code": e.display() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_number_round_trips() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "42");
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":42.0}"#);
        assert_eq!(s.get_raw("A1"), "42");
    }

    #[test]
    fn text_and_booleans() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "hello");
        s.set_cell("A2", "TRUE");
        s.set_cell("A3", "false");
        assert_eq!(s.get_value("A1"), r#"{"kind":"text","value":"hello"}"#);
        assert_eq!(s.get_value("A2"), r#"{"kind":"boolean","value":true}"#);
        assert_eq!(s.get_value("A3"), r#"{"kind":"boolean","value":false}"#);
    }

    #[test]
    fn formula_computes_and_recalcs() {
        let mut s = SpreadsheetSession::new();
        for (a, v) in [("B1", "15"), ("B2", "8"), ("B3", "12"), ("B4", "4"), ("B5", "7")] {
            s.set_cell(a, v);
        }
        s.set_cell("B6", "=SUM(B1:B5)");
        assert_eq!(s.get_value("B6"), r#"{"kind":"number","value":46.0}"#);
        // The formula bar shows the source, not the result.
        assert_eq!(s.get_raw("B6"), "=SUM(B1:B5)");
        // Change a precedent → the total recomputes.
        s.set_cell("B1", "115");
        assert_eq!(s.get_value("B6"), r#"{"kind":"number","value":146.0}"#);
    }

    // ── Multi-sheet workbook ────────────────────────────────────────

    #[test]
    fn multi_sheet_add_switch_cross_sheet_and_per_sheet_raw() {
        let mut s = SpreadsheetSession::new(); // active = Sheet1
        assert_eq!(s.sheet_names(), r#"{"active":0,"sheets":["Sheet1"]}"#);
        // Add a Summary sheet, put a value on it, switch back, reference it.
        assert!(s.add_sheet("Summary")); // now active = Summary (index 1)
        assert_eq!(s.active_sheet(), 1);
        s.set_cell("A1", "10"); // Summary!A1
        assert!(s.set_active_sheet(0)); // back to Sheet1
        s.set_cell("B1", "=Summary!A1*2"); // cross-sheet formula
        assert_eq!(s.get_value("B1"), r#"{"kind":"number","value":20.0}"#);
        // Per-sheet raw echo: Sheet1!B1 shows its source; Summary!A1 is on the
        // other sheet (switch to see it; Sheet1 has no A1).
        assert_eq!(s.get_raw("B1"), "=Summary!A1*2");
        assert_eq!(s.get_raw("A1"), ""); // Sheet1!A1 is empty
        s.set_active_sheet(1);
        assert_eq!(s.get_raw("A1"), "10"); // Summary!A1's echo
        // Edit Summary!A1 → the Sheet1 dependent recomputes (cross-sheet live).
        s.set_cell("A1", "50");
        s.set_active_sheet(0);
        assert_eq!(s.get_value("B1"), r#"{"kind":"number","value":100.0}"#);
    }

    #[test]
    fn multi_sheet_rename_delete_move_and_load() {
        let mut s = SpreadsheetSession::new();
        s.add_sheet("Summary");
        s.set_cell("A1", "7"); // Summary!A1
        s.set_active_sheet(0);
        s.set_cell("B1", "=Summary!A1"); // 7

        // Rename Summary → Totals: the qualifier follows, value holds.
        assert!(s.rename_sheet(1, "Totals"));
        assert_eq!(s.sheet_names(), r#"{"active":0,"sheets":["Sheet1","Totals"]}"#);
        assert_eq!(s.get_value("B1"), r#"{"kind":"number","value":7.0}"#);

        // Save/load round-trips both sheets + the live cross-sheet formula.
        let doc = s.serialize();
        let mut t = SpreadsheetSession::new();
        assert!(t.deserialize(&doc));
        assert_eq!(t.sheet_names(), r#"{"active":0,"sheets":["Sheet1","Totals"]}"#);
        assert_eq!(t.get_value("B1"), r#"{"kind":"number","value":7.0}"#);
        t.set_active_sheet(1);
        t.set_cell("A1", "9"); // Totals!A1
        t.set_active_sheet(0);
        assert_eq!(t.get_value("B1"), r#"{"kind":"number","value":9.0}"#); // live after load

        // Delete Totals: the inbound ref becomes #REF!, only Sheet1 remains.
        assert!(t.delete_sheet(1));
        assert_eq!(t.sheet_names(), r#"{"active":0,"sheets":["Sheet1"]}"#);
        assert_eq!(t.get_value("B1"), r##"{"code":"#REF!","kind":"error"}"##);
        assert!(!t.delete_sheet(0)); // can't delete the last sheet

        // Move: with three sheets, move the first to the end.
        let mut m = SpreadsheetSession::new();
        m.add_sheet("B");
        m.add_sheet("C");
        m.set_active_sheet(0); // active = Sheet1
        assert!(m.move_sheet(0, 2));
        assert_eq!(m.sheet_names(), r#"{"active":2,"sheets":["B","C","Sheet1"]}"#);
    }

    // ── Structural edits: insert / delete rows & columns ────────────

    #[test]
    fn insert_rows_shifts_values_raw_and_formula_refs() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "10");
        s.set_cell("A2", "20");
        s.set_cell("A3", "=SUM(A1:A2)");
        assert_eq!(s.get_value("A3"), r#"{"kind":"number","value":30.0}"#);

        s.insert_rows(1, 1); // a blank row at the top; everything slides down

        assert_eq!(s.get_value("A1"), r#"{"kind":"empty"}"#); // now blank
        assert_eq!(s.get_value("A2"), r#"{"kind":"number","value":10.0}"#); // was A1
        assert_eq!(s.get_value("A4"), r#"{"kind":"number","value":30.0}"#); // SUM moved
        // Echo: the SUM moved to A4 and its range rewrote A1:A2 → A2:A3.
        assert_eq!(s.get_raw("A4"), "=SUM(A2:A3)");
        assert_eq!(s.get_raw("A1"), ""); // nothing there now
    }

    #[test]
    fn delete_cols_makes_dangling_reference_ref_error() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "10");
        s.set_cell("B1", "=A1+1");
        s.delete_cols(1, 1); // delete column A

        // B1 → A1, and its reference A1 (now deleted) → #REF!.
        assert_eq!(s.get_value("A1"), r##"{"code":"#REF!","kind":"error"}"##);
        // The echoed source shows the dangling reference (binary ops are fully
        // parenthesised by the serializer).
        assert_eq!(s.get_raw("A1"), "=(#REF!+1)");
    }

    #[test]
    fn delete_rows_shifts_survivors_up() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "10");
        s.set_cell("A2", "20");
        s.set_cell("A3", "=A2*2");
        s.delete_rows(1, 1); // delete row 1

        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":20.0}"#); // was A2
        assert_eq!(s.get_value("A2"), r#"{"kind":"number","value":40.0}"#); // =A1*2
        assert_eq!(s.get_raw("A2"), "=(A1*2)");
    }

    #[test]
    fn set_get_and_apply_display_format() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "1234.5");
        // No format → General display.
        assert_eq!(s.get_display("A1"), "1234.5");
        assert_eq!(s.get_format("A1"), "");
        // Set a format → get_display applies it; get_format echoes the code.
        s.set_format("A1", "#,##0.00");
        assert_eq!(s.get_format("A1"), "#,##0.00");
        assert_eq!(s.get_display("A1"), "1,234.50");
        // get_value (typed) is unaffected by the format.
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":1234.5}"#);
        // Clearing the format reverts to General.
        s.set_format("A1", "");
        assert_eq!(s.get_format("A1"), "");
        assert_eq!(s.get_display("A1"), "1234.5");
    }

    #[test]
    fn fill_replicates_formula_shifting_refs_and_echoes_source() {
        let mut s = SpreadsheetSession::new();
        for (a, v) in [("A1", "10"), ("A2", "20"), ("A3", "30")] {
            s.set_cell(a, v);
        }
        s.set_cell("B1", "=A1*2"); // 20
        // Fill B1 down into B2:B3 — each tracks its row.
        s.fill("B1", "B2", "B3");
        assert_eq!(s.get_value("B2"), r#"{"kind":"number","value":40.0}"#); // A2*2
        assert_eq!(s.get_value("B3"), r#"{"kind":"number","value":60.0}"#); // A3*2
        // The formula bar echoes the shifted source (binary ops parenthesised).
        assert_eq!(s.get_raw("B3"), "=(A3*2)");
    }

    #[test]
    fn fill_carries_literal_and_clears_from_empty() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "7");
        s.fill("A1", "B1", "C1"); // copy literal right
        assert_eq!(s.get_value("C1"), r#"{"kind":"number","value":7.0}"#);
        assert_eq!(s.get_raw("B1"), "7");
        // Filling from an empty source clears the targets (raw echo too).
        s.set_cell("D1", "99");
        s.fill("Z9", "D1", "D1"); // Z9 is empty
        assert_eq!(s.get_raw("D1"), "");
        assert_eq!(s.get_value("D1"), r#"{"kind":"empty"}"#);
    }

    #[test]
    fn fill_bad_address_is_noop() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "1");
        s.fill("not-an-addr", "B1", "B2"); // no panic, nothing filled
        assert_eq!(s.get_value("B1"), r#"{"kind":"empty"}"#);
    }

    #[test]
    fn copy_paste_shifts_block_and_echoes_shifted_source() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("B1", "5");
        s.set_cell("C1", "=B1*2"); // 10
        s.copy("B1", "C1"); // copy the 1×2 block
        assert!(s.has_clipboard());
        assert!(s.paste("B2")); // paste at B2

        assert_eq!(s.get_value("B2"), r#"{"kind":"number","value":5.0}"#);
        assert_eq!(s.get_value("C2"), r#"{"kind":"number","value":10.0}"#); // B2*2
        // The echo shows the shifted source (binary op parenthesised).
        assert_eq!(s.get_raw("C2"), "=(B2*2)");
        // A copy survives for another paste; the source is untouched.
        assert!(s.has_clipboard());
        assert_eq!(s.get_raw("C1"), "=B1*2");
    }

    #[test]
    fn cut_paste_moves_and_clears_source_echo() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "7");
        s.cut("A1", "A1");
        assert!(s.paste("C1"));
        assert_eq!(s.get_value("C1"), r#"{"kind":"number","value":7.0}"#);
        assert_eq!(s.get_raw("C1"), "7");
        assert_eq!(s.get_value("A1"), r#"{"kind":"empty"}"#); // source value cleared
        assert_eq!(s.get_raw("A1"), ""); // source echo cleared
        // Buffer consumed: a second paste is a no-op.
        assert!(!s.has_clipboard());
        assert!(!s.paste("E1"));
    }

    #[test]
    fn paste_without_copy_is_noop() {
        let mut s = SpreadsheetSession::new();
        assert!(!s.has_clipboard());
        assert!(!s.paste("A1"));
    }

    #[test]
    fn serialize_then_deserialize_round_trips_through_the_facade() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "15");
        s.set_cell("B1", "hi");
        s.set_cell("C1", "=A1*2"); // 30
        s.set_format("A1", "#,##0.00");
        let saved = s.serialize();

        // Load into a fresh facade and confirm values, format, and raw echo.
        let mut loaded = SpreadsheetSession::new();
        assert!(loaded.deserialize(&saved));
        assert_eq!(loaded.get_value("A1"), r#"{"kind":"number","value":15.0}"#);
        assert_eq!(loaded.get_value("C1"), r#"{"kind":"number","value":30.0}"#);
        assert_eq!(loaded.get_raw("A1"), "15"); // literal echo reconstructed
        assert_eq!(loaded.get_raw("C1"), "=A1*2"); // formula echo exact
        assert_eq!(loaded.get_display("A1"), "15.00"); // format survived
        // The formula stays live, not frozen.
        loaded.set_cell("A1", "100");
        assert_eq!(loaded.get_value("C1"), r#"{"kind":"number","value":200.0}"#);
    }

    #[test]
    fn deserialize_bad_json_returns_false() {
        let mut s = SpreadsheetSession::new();
        assert!(!s.deserialize("not json"));
        assert!(!s.deserialize(r#"{"version":99,"sheets":[]}"#));
        // A zero-sheet file loads (returns true) and leaves the facade usable.
        assert!(s.deserialize(r#"{"version":1,"sheets":[]}"#));
        s.set_cell("A1", "1"); // no panic — the pinned sheet was re-created
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":1.0}"#);
    }

    #[test]
    fn undo_redo_walks_the_edit_history() {
        let mut s = SpreadsheetSession::new();
        assert!(!s.can_undo()); // fresh session: nothing to undo
        assert!(!s.can_redo());

        s.set_cell("A1", "1");
        s.set_cell("A1", "2");
        s.set_cell("B1", "=A1*10"); // 20
        assert_eq!(s.get_value("B1"), r#"{"kind":"number","value":20.0}"#);
        assert!(s.can_undo());

        // Undo the formula: B1 is gone, A1 still 2.
        assert!(s.undo());
        assert_eq!(s.get_raw("B1"), "");
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":2.0}"#);
        // Undo A1=2 → back to A1=1.
        assert!(s.undo());
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":1.0}"#);

        // Redo replays A1=2, then the formula (which recomputes live: 20).
        assert!(s.redo());
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":2.0}"#);
        assert!(s.redo());
        assert_eq!(s.get_raw("B1"), "=A1*10");
        assert_eq!(s.get_value("B1"), r#"{"kind":"number","value":20.0}"#);
        assert!(!s.can_redo());

        // Undo back one, then a NEW edit clears the redo branch.
        assert!(s.undo()); // remove B1 again
        assert!(s.can_redo());
        s.set_cell("C1", "9");
        assert!(!s.can_redo(), "a fresh edit forks history, dropping redo");

        // A loaded formula stays live after undo: A1 still feeds nothing now, but
        // editing a precedent of a *restored* formula recomputes it.
        assert!(s.undo()); // undo C1
        assert!(s.undo()); // undo A1=2 → A1=1 (B1 already gone)
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":1.0}"#);
    }

    #[test]
    fn no_op_edits_do_not_pollute_history() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "5");
        assert!(s.can_undo());
        let depth_before = s.undo_stack.len();

        // A failed set (bad address), a copy (clipboard only — no cell change),
        // and a fill from an empty source into an empty target all leave the
        // document unchanged → no new undo checkpoint.
        s.set_cell("not-an-address", "1");
        s.copy("A1", "A1");
        s.fill("X1", "X2", "X2"); // empty → empty: a true no-op
        // Re-set A1 to the SAME value: still a no-op for the document.
        s.set_cell("A1", "5");
        assert_eq!(s.undo_stack.len(), depth_before, "no-ops added no history");

        // One real edit adds exactly one checkpoint.
        s.set_cell("A1", "6");
        assert_eq!(s.undo_stack.len(), depth_before + 1);
        assert!(s.undo());
        assert_eq!(s.get_value("A1"), r#"{"kind":"number","value":5.0}"#);
    }

    #[test]
    fn undo_redo_on_empty_history_is_a_safe_noop() {
        let mut s = SpreadsheetSession::new();
        assert!(!s.undo());
        assert!(!s.redo());
        // History survives reads; still nothing to do.
        let _ = s.get_value("A1");
        assert!(!s.undo());
    }

    #[test]
    fn operator_precedence() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "3");
        s.set_cell("A2", "5");
        s.set_cell("C1", "=A1+A2*2"); // 3 + 10
        assert_eq!(s.get_value("C1"), r#"{"kind":"number","value":13.0}"#);
    }

    #[test]
    fn divide_by_zero_is_an_error_value() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "=1/0");
        assert_eq!(s.get_value("A1"), r##"{"code":"#DIV/0!","kind":"error"}"##);
    }

    #[test]
    fn error_propagates_through_dependents() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "=1/0");
        s.set_cell("A2", "=A1+1");
        assert_eq!(s.get_value("A2"), r##"{"code":"#DIV/0!","kind":"error"}"##);
    }

    #[test]
    fn unparseable_formula_shows_value_error_but_keeps_source() {
        let mut s = SpreadsheetSession::new();
        // `set_cell` itself succeeds (ok:true); the cell shows #VALUE!.
        let out = s.set_cell("A1", "=1 +* 2");
        assert_eq!(out, r#"{"ok":true}"#);
        assert_eq!(s.get_value("A1"), r##"{"code":"#VALUE!","kind":"error"}"##);
        assert_eq!(s.get_raw("A1"), "=1 +* 2"); // source preserved
    }

    #[test]
    fn bad_address_is_reported() {
        let mut s = SpreadsheetSession::new();
        let out = s.set_cell("not-an-address", "1");
        assert!(out.contains(r#""ok":false"#), "got {out}");
        // Reading a bad address yields an error object, not a panic.
        assert_eq!(s.get_value("???"), r##"{"code":"#REF!","kind":"error"}"##);
    }

    #[test]
    fn clearing_a_cell_removes_it_from_values() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "5");
        s.set_cell("A1", "   "); // whitespace clears
        assert_eq!(s.get_raw("A1"), "");
        assert_eq!(s.get_values(), "{}");
    }

    #[test]
    fn get_values_maps_a1_to_computed_values() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "2");
        s.set_cell("A2", "=A1*3");
        let out = s.get_values();
        // Order within a JSON object isn't guaranteed; check membership.
        assert!(out.contains(r#""A1":{"kind":"number","value":2.0}"#), "{out}");
        assert!(out.contains(r#""A2":{"kind":"number","value":6.0}"#), "{out}");
    }

    #[test]
    fn text_values_are_json_escaped() {
        // A label containing JSON metacharacters must not break the JSON
        // (serde_json escapes it); this is the injection-safety guarantee the
        // JS host relies on.
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", r#"a"b<c>"#);
        assert_eq!(s.get_value("A1"), r#"{"kind":"text","value":"a\"b<c>"}"#);
    }

    #[test]
    fn oversized_range_surfaces_ref_not_oom() {
        // The engine's range cap flows through the facade as a #REF! value.
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "=SUM(A1:XFD1048576)");
        assert_eq!(s.get_value("A1"), r##"{"code":"#REF!","kind":"error"}"##);
    }

    // ── Viewport facade ──────────────────────────────────────────────

    #[test]
    fn get_window_returns_dense_row_major_json() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "15");
        s.set_cell("B1", "3");
        s.set_cell("C1", "=SUM(A1:B1)"); // 18
        // Window A1:C1 — one row, three columns, computed values, dense.
        let out = s.get_window(1, 1, 1, 3);
        assert_eq!(
            out,
            r#"{"col0":1,"cols":3,"row0":1,"rows":1,"values":[[{"kind":"number","value":15.0},{"kind":"number","value":3.0},{"kind":"number","value":18.0}]]}"#
        );
    }

    #[test]
    fn get_window_includes_blanks_and_rejects_bad_requests() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "1"); // B1 left empty
        let out = s.get_window(1, 1, 1, 2);
        assert!(out.contains(r#"{"kind":"number","value":1.0}"#));
        assert!(out.contains(r#"{"kind":"empty"}"#)); // blank cell present, not omitted
        // 0-coord / oversized → an error object, never a panic.
        assert_eq!(s.get_window(0, 0, 10, 10), r##"{"error":"#REF!"}"##);
        assert_eq!(s.get_window(1, 1, 1000, 1000), r##"{"error":"#REF!"}"##);
    }

    #[test]
    fn get_display_window_returns_formatted_strings_row_major() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "1234.5");
        s.set_format("A1", "#,##0.00"); // → "1,234.50"
        s.set_cell("B1", "0.25");
        s.set_format("B1", "0%"); // → "25%"
        s.set_cell("C1", "hi"); // text, General
        // Window A1:C1 — one row, three columns, display strings, dense.
        let out = s.get_display_window(1, 1, 1, 3);
        assert_eq!(
            out,
            r#"{"cells":[["1,234.50","25%","hi"]],"col0":1,"cols":3,"row0":1,"rows":1}"#
        );
        // A blank region comes back as "" cells (included, not omitted): row 2,
        // columns A:B → one row, two empty strings.
        let out2 = s.get_display_window(2, 1, 2, 2);
        assert_eq!(
            out2,
            r#"{"cells":[["",""]],"col0":1,"cols":2,"row0":2,"rows":1}"#
        );
        // 0-coord / oversized → an error object, never a panic.
        assert_eq!(s.get_display_window(0, 0, 10, 10), r##"{"error":"#REF!"}"##);
        assert_eq!(
            s.get_display_window(1, 1, 1000, 1000),
            r##"{"error":"#REF!"}"##
        );
    }

    #[test]
    fn used_range_reports_extent_or_null() {
        let mut s = SpreadsheetSession::new();
        assert_eq!(s.used_range(), "null");
        s.set_cell("A1", "1");
        s.set_cell("Z100", "2");
        assert_eq!(
            s.used_range(),
            r#"{"maxCol":26,"maxRow":100,"minCol":1,"minRow":1}"#
        );
    }

    #[test]
    fn column_letters_matches_excel() {
        let s = SpreadsheetSession::new();
        assert_eq!(s.column_letters(1), "A");
        assert_eq!(s.column_letters(26), "Z");
        assert_eq!(s.column_letters(27), "AA");
        assert_eq!(s.column_letters(703), "AAA");
    }

    #[test]
    fn changed_since_reports_delta_then_current_revision() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "1");
        s.set_cell("B1", "=A1*10");
        let snap = s.current_revision();
        s.set_cell("A1", "9"); // A1 + dependent B1
        let out = s.changed_since(snap);
        assert!(out.contains("\"changed\""));
        assert!(out.contains("\"A1\""));
        assert!(out.contains("\"B1\""));
        assert!(!out.contains("\"stale\""));
    }

    #[test]
    fn sort_range_reorders_rows_and_keeps_the_raw_echo_in_step() {
        let mut s = SpreadsheetSession::new();
        // A = key, B = a formula on its own row's A. Sort by A ascending; rows
        // move and B's relative ref must shift with each row.
        s.set_cell("A1", "30");
        s.set_cell("A2", "10");
        s.set_cell("A3", "20");
        s.set_cell("B1", "=A1*2");
        s.set_cell("B2", "=A2*2");
        s.set_cell("B3", "=A3*2");
        assert!(s.sort_range("A1", "B3", 1, true));
        // Keys sorted 10,20,30.
        assert!(s.get_value("A1").contains("10"));
        assert!(s.get_value("A2").contains("20"));
        assert!(s.get_value("A3").contains("30"));
        // Each B is its row's A*2 (the moved formula's ref shifted with its row).
        assert!(s.get_value("B1").contains("20"));
        assert!(s.get_value("B2").contains("40"));
        assert!(s.get_value("B3").contains("60"));
        // The raw echo moved too: selecting B1 shows the shifted source. The
        // printer fully-parenthesizes binary ops on re-emit (like fill's echo),
        // so strip parens before comparing to the logical source.
        let bare = |raw: String| raw.replace(['(', ')'], "");
        assert_eq!(bare(s.get_raw("B1")), "=A1*2");
        assert_eq!(bare(s.get_raw("B3")), "=A3*2");
    }

    #[test]
    fn sort_range_rejects_bad_args() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "2");
        s.set_cell("A2", "1");
        assert!(!s.sort_range("nope", "A2", 1, true)); // malformed address
        assert!(!s.sort_range("A1", "A1", 1, true)); // single-row range
        assert!(!s.sort_range("A1", "A2", 9, true)); // key_col outside range
    }

    #[test]
    fn find_all_and_replace_all_over_the_facade() {
        let mut s = SpreadsheetSession::new();
        s.set_cell("A1", "100");
        s.set_cell("A2", "100");
        s.set_cell("B1", "=A1+1"); // displays 101
        // find by computed value: "10" is in 100 (A1, A2) and 101 (B1 display).
        let by_val = s.find_all("10", false, true);
        assert!(by_val.contains("\"A1\"") && by_val.contains("\"A2\"") && by_val.contains("\"B1\""));
        // find by source: "A1" only in B1's formula text.
        let by_src = s.find_all("A1", true, true);
        assert!(by_src.contains("\"B1\"") && !by_src.contains("\"A1\""));
        // empty query → empty matches.
        assert_eq!(s.find_all("", false, true), "{\"matches\":[]}");
        // replace the literal 100 → 7 in the two number cells (count 2); the raw
        // echo resyncs so get_raw shows the new source.
        assert_eq!(s.replace_all("100", "7", true), 2);
        assert_eq!(s.get_raw("A1"), "7");
        assert!(s.get_value("A1").contains("7"));
        // replace A1 → A2 in the formula source; it re-parses + recomputes (=A2+1 = 8).
        assert_eq!(s.replace_all("A1", "A2", true), 1);
        assert!(s.get_value("B1").contains("8"));
        // no-match / empty query → 0.
        assert_eq!(s.replace_all("zzz", "q", true), 0);
        assert_eq!(s.replace_all("", "q", true), 0);
    }

    #[test]
    fn column_width_and_row_height_through_the_session() {
        let mut s = SpreadsheetSession::new();
        // Unset → 0.0 (the host-default sentinel).
        assert_eq!(s.column_width(3), 0.0);
        assert_eq!(s.row_height(2), 0.0);
        // Set + read back; a bad value is rejected (false), leaving it unset.
        assert!(s.set_column_width(3, 140.0));
        assert!(s.set_row_height(2, 40.0));
        assert_eq!(s.column_width(3), 140.0);
        assert_eq!(s.row_height(2), 40.0);
        assert!(!s.set_column_width(4, f64::NAN)); // rejected
        assert_eq!(s.column_width(4), 0.0);
        // Bulk JSON for a viewport range, sorted.
        s.set_column_width(5, 90.0);
        // JSON object keys are serde-sorted (alphabetical): col<w, h<row.
        assert_eq!(s.column_widths(3, 6), r#"[{"col":3,"w":140.0},{"col":5,"w":90.0}]"#);
        assert_eq!(s.row_heights(1, 5), r#"[{"h":40.0,"row":2}]"#);
        // A resize is undoable (routed through `mutate`): undo restores the width.
        assert!(s.can_undo());
        assert!(s.undo());
        assert_eq!(s.column_width(5), 0.0); // last set (col 5) undone
        assert_eq!(s.column_width(3), 140.0); // earlier one survives
        // Clear → back to the default sentinel.
        assert!(s.clear_column_width(3));
        assert_eq!(s.column_width(3), 0.0);
    }

    #[test]
    fn widths_survive_save_load_and_shift_on_insert_col() {
        let mut s = SpreadsheetSession::new();
        s.set_column_width(3, 140.0);
        s.set_row_height(2, 40.0);
        let snap = s.serialize();
        // Mutate, then restore — the saved sizes come back.
        s.set_column_width(3, 999.0);
        assert!(s.deserialize(&snap));
        assert_eq!(s.column_width(3), 140.0);
        assert_eq!(s.row_height(2), 40.0);
        // Insert a column at B (2): C's width slides to D (4).
        s.insert_cols(2, 1);
        assert_eq!(s.column_width(3), 0.0);
        assert_eq!(s.column_width(4), 140.0);
    }
}
