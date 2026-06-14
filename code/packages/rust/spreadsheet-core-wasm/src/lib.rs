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
use spreadsheet_core::{CellAddress, CellValue, SheetId, SpreadsheetError, Workbook};

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
}
