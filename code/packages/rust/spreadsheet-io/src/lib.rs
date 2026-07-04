//! # `spreadsheet-io` — one spreadsheet core, many file formats
//!
//! This crate is the **adapter layer** that unifies every spreadsheet file
//! format onto a single in-memory model. See `code/specs/SSIO01-spreadsheet-io.md`
//! for the full design.
//!
//! ## The problem it solves
//!
//! The repo grew five *disconnected* spreadsheet models — the live
//! [`spreadsheet_core`] engine (what VisiCalc computes on) plus a private
//! `Workbook`/`Cell` type inside each of the four `.xlsx`/`.xls` reader and
//! writer crates. None could talk to another. This crate makes
//! [`spreadsheet_core::Workbook`] the **single hub** and provides the one
//! conversion layer that turns file bytes into a live workbook and back:
//!
//! ```text
//!   .xlsx ──load_xlsx──┐                              ┌──save_xlsx──▶ .xlsx
//!                      ├─▶  spreadsheet_core::Workbook ─┤
//!   .xls  ─(SSIO02)────┘     (THE model; VisiCalc      └─(SSIO02)────▶ .xls
//!                             computes here)
//! ```
//!
//! It is deliberately the *only* crate that depends on both the engine and the
//! file-format codecs, so the engine stays oblivious to file formats and each
//! codec stays a small, faithful reader/writer.
//!
//! ## Example — round-trip a live workbook through `.xlsx`
//!
//! ```
//! use spreadsheet_core::{CellAddress, CellValue, Workbook};
//! use spreadsheet_io::{load_xlsx, save_xlsx};
//!
//! // Build a workbook the way VisiCalc would: two numbers and a live SUM.
//! let mut wb = Workbook::new();
//! let s = wb.add_sheet("Sheet1");
//! wb.set_value(s, CellAddress::new(1, 1), CellValue::Number(10.0));
//! wb.set_value(s, CellAddress::new(1, 2), CellValue::Number(20.0));
//! wb.set_formula(s, CellAddress::new(1, 3), "=SUM(A1:B1)").unwrap();
//! wb.recalc_all();
//!
//! // Save to .xlsx bytes, then open them back up.
//! let bytes = save_xlsx(&wb);
//! assert_eq!(&bytes[..2], b"PK"); // a ZIP, hence an OPC package
//! let reopened = load_xlsx(&bytes).unwrap();
//!
//! // The computed value survived, and C1 is *still a formula* (not frozen).
//! let s2 = reopened.sheet_id("Sheet1").unwrap();
//! assert_eq!(reopened.get_value(s2, CellAddress::new(1, 3)),
//!            Some(CellValue::Number(30.0)));
//! assert!(reopened.cell_is_formula(s2, CellAddress::new(1, 3)));
//! ```

#![forbid(unsafe_code)]

use coding_adventures_xlsx_eval::open_and_evaluate;
use coding_adventures_xlsx_writer::{write_xlsx, Workbook as XlsxOut};
use spreadsheet_core::{CellAddress, CellValue, SpreadsheetError, Workbook};

/// The engine workbook, re-exported so callers can name the type this crate
/// loads into and saves from without a separate dependency on `spreadsheet-core`.
pub use spreadsheet_core::Workbook as CoreWorkbook;

// ===========================================================================
// Errors
// ===========================================================================

/// Something went wrong loading a file into the engine.
///
/// Saving is infallible (it is pure computation over an in-memory model that is
/// trusted to be well-formed), so only the load path has an error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoError {
    /// The `.xlsx` bytes could not be opened or evaluated — a bad ZIP, malformed
    /// XML, an unresolvable relationship, or a cell reference the reader could
    /// not parse. Carries the underlying message for diagnostics.
    Xlsx(String),
    /// The `.xls` (legacy BIFF8) bytes could not be opened — not an OLE2
    /// compound file, a missing/short workbook stream, or a malformed record.
    /// Carries the underlying message for diagnostics.
    Xls(String),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Xlsx(msg) => write!(f, "failed to load .xlsx: {msg}"),
            IoError::Xls(msg) => write!(f, "failed to load .xls: {msg}"),
        }
    }
}

impl std::error::Error for IoError {}

// ===========================================================================
// Load: .xlsx bytes → live engine workbook
// ===========================================================================

/// Load a `.xlsx` file's bytes into a live [`spreadsheet_core::Workbook`].
///
/// This is a thin, honest wrapper over the read stack the repo already has:
///
/// ```text
/// bytes ─▶ spreadsheetml::open_workbook   (zip → xml → OPC → SpreadsheetML)
///       ─▶ xlsx-eval::evaluate_workbook   (formulas via set_formula,
///                                           literals via set_value, recalc_all)
///       ─▶ spreadsheet_core::Workbook
/// ```
///
/// Crucially, formulas come back as **formulas** (the `<f>` body is fed to the
/// engine's parser), not as frozen numbers — so the result is immediately
/// editable and [`save_xlsx`] can write the formula back out. A formula the
/// engine cannot parse falls back to its cached value (handled inside
/// `xlsx-eval`); the load as a whole still succeeds.
///
/// # Errors
///
/// Returns [`IoError::Xlsx`] if the bytes are not a readable `.xlsx`.
pub fn load_xlsx(bytes: &[u8]) -> Result<Workbook, IoError> {
    open_and_evaluate(bytes).map_err(|e| IoError::Xlsx(e.to_string()))
}

// ===========================================================================
// Save: live engine workbook → .xlsx bytes
// ===========================================================================

/// Serialize a live [`spreadsheet_core::Workbook`] to the bytes of a `.xlsx`
/// file.
///
/// The whole conversion is driven off the unified core model through four
/// read accessors — `sheet_names` / `populated_cells` / `cell_is_formula` /
/// (`cell_source_text` + `get_value`) — so nothing about the engine's internals
/// leaks and any workbook the engine can hold can be written.
///
/// It walks each sheet's **populated cells sparsely** (`populated_cells`), not
/// the dense bounding box: a workbook with a cell at `A1` and one at
/// `XFD1048576` is written in two steps, not seventeen billion.
///
/// ## Per-cell mapping
///
/// | Cell in the engine | Written to `.xlsx` |
/// |--------------------|--------------------|
/// | formula, cached `Number(n)`     | `<f>body</f><v>n</v>` |
/// | formula, cached `Boolean(b)`    | `<f>body</f><v>1|0</v>` |
/// | formula, cached `Text`/`Error`  | the **computed value** as a literal¹ |
/// | literal `Number(n)`             | numeric `<v>` |
/// | literal `Text(s)`               | shared-string `<v>` |
/// | literal `Boolean(b)`            | numeric `1`/`0`¹ |
/// | literal `Error(e)`              | its display text as a string |
/// | empty                           | omitted |
///
/// ¹ Documented fidelity limits of the current `.xlsx` writer's value model
/// (its cached slot is an `f64`; it has no `t="b"`). Numbers, text, and
/// numeric-result formulas — a whole VisiCalc-authored sheet — round-trip
/// exactly. See the spec for the full list.
///
/// The output is deterministic: the same workbook yields the same bytes.
pub fn save_xlsx(wb: &Workbook) -> Vec<u8> {
    let mut out = XlsxOut::new();

    // Sheets in engine order (add_sheet hands out dense 0,1,2… ids that match
    // insertion order, and sheet_names() returns that same order).
    for name in wb.sheet_names() {
        let Some(sid) = wb.sheet_id(name) else {
            continue; // unreachable: the name came straight from sheet_names()
        };
        let ws = out.add_sheet(name);

        // Walk ONLY the populated cells (sparse, sorted by row/col), never the
        // dense bounding box — a sheet can be tiny yet span a huge range.
        for addr in wb.populated_cells(sid) {
            let a1 = addr.to_a1(); // relative address → plain "A1", no `$`
            let value = wb.get_value(sid, addr).unwrap_or(CellValue::Empty);

            if wb.cell_is_formula(sid, addr) {
                // A formula: write `<f>` + its cached numeric result when we can;
                // otherwise fall back to the computed value as a literal (the
                // writer's cache is f64-only — see the mapping table).
                let text = wb.cell_source_text(sid, addr);
                let body = text.strip_prefix('=').unwrap_or(&text);
                match value {
                    CellValue::Number(n) => ws.set_formula(&a1, body, n),
                    CellValue::Boolean(b) => ws.set_formula(&a1, body, if b { 1.0 } else { 0.0 }),
                    CellValue::Text(s) => ws.set_string(&a1, &s),
                    CellValue::Error(e) => ws.set_string(&a1, e.display()),
                    CellValue::Empty => {}
                }
            } else {
                match value {
                    CellValue::Number(n) => ws.set_number(&a1, n),
                    CellValue::Text(s) => ws.set_string(&a1, &s),
                    CellValue::Boolean(b) => ws.set_number(&a1, if b { 1.0 } else { 0.0 }),
                    CellValue::Error(e) => ws.set_string(&a1, e.display()),
                    CellValue::Empty => {}
                }
            }
        }
    }

    write_xlsx(&out)
}

// ===========================================================================
// Legacy .xls (BIFF8) — load & save
// ===========================================================================
//
// The `.xls` codecs are less capable than the `.xlsx` ones, and the bridge is
// honest about it:
//
// - The `.xls` **reader** decodes a formula cell's *cached value* but not its
//   expression, so a `.xls` formula loads as a plain value (the formula is
//   lost). This is a reader limitation, not a bridge choice.
// - The `.xls` **writer** models only numbers and strings — no formulas, no
//   booleans, no error sentinels — so save writes computed values (bools as
//   1/0, errors as their display text). BIFF cell addresses are `u16`, so a
//   cell beyond row/col 65535 is skipped by the writer.
//
// Numbers and text round-trip exactly. See `code/specs/SSIO02-spreadsheet-io-xls.md`.

/// Map a BIFF8 error code (the `u8` the `.xls` reader surfaces) to the engine's
/// typed [`SpreadsheetError`]. The codes are fixed by [MS-XLS]; an unrecognised
/// one falls back to `#VALUE!` (a generic "bad value") rather than being dropped.
fn biff_error_to_core(code: u8) -> SpreadsheetError {
    match code {
        0x00 => SpreadsheetError::Null,         // #NULL!
        0x07 => SpreadsheetError::DivZero,      // #DIV/0!
        0x0F => SpreadsheetError::Value,        // #VALUE!
        0x17 => SpreadsheetError::Ref,          // #REF!
        0x1D => SpreadsheetError::Name,         // #NAME?
        0x24 => SpreadsheetError::Num,          // #NUM!
        0x2A => SpreadsheetError::NotAvailable, // #N/A
        _ => SpreadsheetError::Value,
    }
}

/// Convert one decoded `.xls` cell value into an engine [`CellValue`]. A formula
/// cell carries only its *cached* result (the reader does not decode the
/// expression), so we unwrap that cache and store it as a literal.
///
/// The `Formula { cached }` unwrap is written as a **loop, not recursion**: the
/// `xls` reader only ever puts a leaf value in `cached`, so in practice the loop
/// runs once — but iterating means even a hypothetical deeply-nested cache (which
/// the type permits) costs no stack, so untrusted input can never overflow here.
fn xls_value_to_core(v: &xls::CellValue) -> CellValue {
    let mut v = v;
    while let xls::CellValue::Formula { cached } = v {
        v = cached;
    }
    match v {
        xls::CellValue::Number(n) => CellValue::Number(*n),
        xls::CellValue::Text(s) => CellValue::Text(s.clone()),
        xls::CellValue::Bool(b) => CellValue::Boolean(*b),
        xls::CellValue::Error(code) => CellValue::Error(biff_error_to_core(*code)),
        xls::CellValue::Blank => CellValue::Empty,
        // Unreachable: the loop above stripped every Formula layer.
        xls::CellValue::Formula { .. } => CellValue::Empty,
    }
}

/// Load a legacy `.xls` (BIFF8) file's bytes into a live
/// [`spreadsheet_core::Workbook`].
///
/// ```text
/// bytes ─▶ xls::open_xls   (OLE2/CFB → BIFF records → typed cells, 0-based)
///       ─▶ spreadsheet_core::Workbook   (addresses shifted to 1-based)
/// ```
///
/// The `.xls` reader recovers each cell's *value* (including a formula's cached
/// result), but not formula expressions — so formulas arrive as literals. Blank
/// cells are skipped.
///
/// # Errors
///
/// Returns [`IoError::Xls`] if the bytes are not a readable `.xls`.
pub fn load_xls(bytes: &[u8]) -> Result<Workbook, IoError> {
    let src = xls::open_xls(bytes).map_err(|e| IoError::Xls(e.to_string()))?;

    let mut wb = Workbook::new();
    for sheet in src.sheets() {
        let sid = wb.add_sheet(sheet.name.clone());
        for cell in sheet.cells() {
            let value = xls_value_to_core(&cell.value);
            if value == CellValue::Empty {
                continue; // blank cells carry no content
            }
            // .xls is 0-based; the engine is 1-based.
            let addr = CellAddress::new(cell.row + 1, cell.col + 1);
            wb.set_value(sid, addr, value);
        }
    }
    wb.recalc_all();
    Ok(wb)
}

/// Serialize a live [`spreadsheet_core::Workbook`] to legacy `.xls` (BIFF8)
/// bytes.
///
/// Like [`save_xlsx`], it walks each sheet's populated cells **sparsely**
/// (`populated_cells`). But the `.xls` writer's value model is smaller — only
/// numbers and strings — so every cell is written as its **computed value**:
///
/// | Cell in the engine | Written to `.xls` |
/// |--------------------|-------------------|
/// | `Number(n)` (literal or formula cache) | numeric cell |
/// | `Text(s)`                              | shared-string cell |
/// | `Boolean(b)`                           | numeric `1`/`0` |
/// | `Error(e)`                             | its display text as a string |
/// | empty                                  | omitted |
///
/// Formulas are therefore **not preserved** in `.xls` (their computed result is
/// written); for formula fidelity, use [`save_xlsx`]. Cells beyond BIFF's `u16`
/// address limit (row/col 65535) are skipped by the writer. Output is
/// deterministic.
pub fn save_xls(wb: &Workbook) -> Vec<u8> {
    let mut out = xls_writer::Workbook::new();

    for name in wb.sheet_names() {
        let Some(sid) = wb.sheet_id(name) else {
            continue;
        };
        let ws = out.add_sheet(name);

        for addr in wb.populated_cells(sid) {
            // The engine is 1-based; the .xls writer is 0-based. populated_cells
            // never yields row/col 0, so these subtractions cannot underflow.
            let row = addr.row - 1;
            let col = addr.col - 1;
            match wb.get_value(sid, addr).unwrap_or(CellValue::Empty) {
                CellValue::Number(n) => ws.set_number(row, col, n),
                CellValue::Text(s) => ws.set_string(row, col, &s),
                CellValue::Boolean(b) => ws.set_number(row, col, if b { 1.0 } else { 0.0 }),
                CellValue::Error(e) => ws.set_string(row, col, e.display()),
                CellValue::Empty => {}
            }
        }
    }

    xls_writer::write_xls(&out)
}

#[cfg(test)]
mod tests;
