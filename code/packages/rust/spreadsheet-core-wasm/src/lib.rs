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
use spreadsheet_core::{
    column_index_to_letters, CellAddress, CellValue, ChangeSet, SheetId, SpreadsheetError,
    StructuralEdit, Workbook,
};

/// A single-sheet spreadsheet session with a JSON boundary.
///
/// The original VisiCalc was single-sheet, and that is all the web demos need,
/// so this facade pins one sheet and addresses cells by bare A1 (`"B6"`). The
/// underlying [`Workbook`] is multi-sheet; a richer facade could expose that
/// later without changing this one.
pub struct SpreadsheetSession {
    wb: Workbook,
    sheet: SheetId,
    /// What was literally typed into each cell — the source of truth for
    /// [`get_raw`](Self::get_raw), independent of the engine's internals.
    raw: HashMap<CellAddress, String>,
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
        }
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
        match catch_unwind(AssertUnwindSafe(|| self.set_cell_inner(a1, raw))) {
            Ok(Ok(())) => json!({ "ok": true }).to_string(),
            Ok(Err(msg)) => json!({ "ok": false, "error": msg }).to_string(),
            Err(_) => json!({ "ok": false, "error": "internal error" }).to_string(),
        }
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
        if let Ok(addr) = CellAddress::parse(a1) {
            self.wb.set_format(self.sheet, addr, code);
        }
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
}
