//! # `coding-adventures-xlsx-eval` — recompute an `.xlsx`'s formulas
//!
//! This is milestone **M5** of the OOXML effort (see `code/specs/SML03`). It is
//! the bridge between two crates that, by design, know nothing about each other:
//!
//! ```text
//! bytes → … → spreadsheetml (M3)  → typed grid (cells + formula TEXT)
//!                                        │
//!                                        ▼   xlsx-eval (M5, HERE)
//!                              spreadsheet_core::Workbook (formulas RECOMPUTED)
//! ```
//!
//! * [`coding_adventures_spreadsheetml`] (M3) reads the `.xlsx` bytes into a
//!   `Workbook → Sheet → Cell` model. A formula cell carries its `<f>` text
//!   **and** its *cached* `<v>` value; M3 never evaluates.
//! * [`spreadsheet_core`] is a complete, self-contained formula engine — cell
//!   model, Pratt formula parser, dependency DAG, recalc, and dispatch into the
//!   Layer-1 math cores (`statistics-core` supplies `SUM`, etc.). It knows
//!   nothing about ZIP / XML / `.xlsx`.
//!
//! `xlsx-eval` depends on both, **modifies neither**, and is entirely opt-in: a
//! caller who only wants the raw grid uses M3 alone and never pays for the
//! engine. Reading bytes and evaluating arithmetic are different concerns, so
//! they live in different crates; this one is the thin data-shape adapter that
//! joins them.
//!
//! ## What "evaluate" means here
//!
//! On open, we **ignore the cached `<v>`** of every formula cell, hand the
//! formula *text* to the engine, and let the engine's own parser + recalc
//! produce the value. This is exactly how a computing spreadsheet host behaves:
//! the cached values on disk are a courtesy for viewers that cannot compute; a
//! host that *can* compute recalculates from the formulas.
//!
//! ```
//! use coding_adventures_xlsx_eval::{evaluate_workbook, computed_value};
//! use spreadsheet_core::CellValue;
//!
//! # fn demo(sml: &coding_adventures_spreadsheetml::Workbook) {
//! let core = evaluate_workbook(sml).unwrap();
//! // A formula cell's value is what the ENGINE computed, not the cached <v>.
//! let v = computed_value(&core, "Revenue", "B2");
//! assert_eq!(v, Some(CellValue::Number(1000.0)));
//! # }
//! ```
//!
//! ## Ownership of formula semantics
//!
//! This crate owns **none** of the arithmetic. Every question of "what does
//! `SUM(B1:B1)` mean, what is its precedence, how does a range expand, what
//! happens on a cycle or an oversized range" is answered inside
//! `spreadsheet-core`. The adapter only reshapes data:
//!
//! * M3 [`Value`](coding_adventures_spreadsheetml::Value) → core
//!   [`CellValue`](spreadsheet_core::CellValue),
//! * M3 A1 string → core [`CellAddress`](spreadsheet_core::CellAddress),
//! * M3 formula text → the engine's
//!   [`set_formula`](spreadsheet_core::Workbook::set_formula).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use coding_adventures_spreadsheetml::{
    open_workbook, parse_a1_ref, Value as SmlValue, Workbook as SmlWorkbook, XlsxError,
};
use spreadsheet_core::{CellAddress, CellValue, SpreadsheetError, Workbook as CoreWorkbook};

// ===========================================================================
// Errors & diagnostics
// ===========================================================================

/// Everything that can make *opening or shaping* a workbook fail outright.
///
/// Note what is **not** here: a formula that fails to *parse* is not an
/// `EvalError`. A single malformed formula must not sink the whole workbook, so
/// it is handled gracefully (the cell keeps its cached value) and surfaced as a
/// non-fatal [`FormulaDiagnostic`] instead. `EvalError` is reserved for
/// failures that mean we could not build a workbook at all.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// The bytes could not be opened as an `.xlsx` — wraps the M3 reader error.
    Open(XlsxError),
    /// A cell's `reference` was not a parseable A1 string (e.g. `""` or junk).
    /// Carries the offending reference. In practice M3 only keeps cells whose
    /// ref parses, so this is a defensive guard rather than an expected path.
    BadReference(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Open(e) => write!(f, "could not open workbook: {e}"),
            EvalError::BadReference(r) => write!(f, "unparseable cell reference {r:?}"),
        }
    }
}

impl std::error::Error for EvalError {}

impl From<XlsxError> for EvalError {
    fn from(e: XlsxError) -> Self {
        EvalError::Open(e)
    }
}

/// A single non-fatal formula failure: a cell whose formula text the engine's
/// parser rejected. The cell was left holding its cached value (as a literal),
/// so the workbook is still usable; this records *where* and *why* for a caller
/// that wants to warn the user or log it.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaDiagnostic {
    /// The sheet name the cell is on.
    pub sheet: String,
    /// The A1 reference of the cell, e.g. `"B2"`.
    pub reference: String,
    /// The formula text that failed to parse.
    pub formula: String,
    /// The parser's error message.
    pub message: String,
}

/// The outcome of an evaluation: the hydrated engine workbook plus any
/// non-fatal formula diagnostics collected along the way.
///
/// Most callers want only the workbook; [`evaluate_workbook`] returns that
/// directly. Use [`evaluate_workbook_verbose`] when you also need the
/// diagnostics.
///
/// (Not `Debug` — the engine's `Workbook` is intentionally not `Debug`; inspect
/// [`diagnostics`](Self::diagnostics) directly, which is.)
pub struct Evaluation {
    /// The recomputed engine workbook.
    pub workbook: CoreWorkbook,
    /// Formula cells whose text the engine could not parse (each fell back to
    /// its cached value). Empty on a clean workbook.
    pub diagnostics: Vec<FormulaDiagnostic>,
}

// ===========================================================================
// Value / error conversion
// ===========================================================================

/// Map an M3 [`Value`](SmlValue) to a `spreadsheet-core`
/// [`CellValue`]. A one-to-one reshape — see the spec's truth table.
pub fn sml_value_to_core(v: &SmlValue) -> CellValue {
    match v {
        SmlValue::Number(n) => CellValue::Number(*n),
        SmlValue::Text(s) => CellValue::Text(s.clone()),
        SmlValue::Bool(b) => CellValue::Boolean(*b),
        SmlValue::Empty => CellValue::Empty,
        SmlValue::Error(s) => CellValue::Error(parse_error_text(s)),
    }
}

/// Map an on-disk error *string* (`"#DIV/0!"`, `"#N/A"`, …) to the engine's
/// [`SpreadsheetError`] sentinel. An unrecognised string is treated as
/// `#VALUE!` — a safe, non-panicking default (a workbook should never crash the
/// adapter just because a producer wrote an error code we don't model).
pub fn parse_error_text(s: &str) -> SpreadsheetError {
    match s.trim() {
        "#DIV/0!" => SpreadsheetError::DivZero,
        "#N/A" => SpreadsheetError::NotAvailable,
        "#NAME?" => SpreadsheetError::Name,
        "#NUM!" => SpreadsheetError::Num,
        "#REF!" => SpreadsheetError::Ref,
        "#VALUE!" => SpreadsheetError::Value,
        "#NULL!" => SpreadsheetError::Null,
        _ => SpreadsheetError::Value,
    }
}

// ===========================================================================
// The bridge
// ===========================================================================

/// Recompute every formula in an already-parsed M3 workbook, returning the
/// hydrated engine workbook. Discards any non-fatal formula diagnostics; use
/// [`evaluate_workbook_verbose`] to keep them.
pub fn evaluate_workbook(sml: &SmlWorkbook) -> Result<CoreWorkbook, EvalError> {
    Ok(evaluate_workbook_verbose(sml)?.workbook)
}

/// Recompute every formula in an already-parsed M3 workbook, returning both the
/// engine workbook and the list of formula cells the engine could not parse
/// (each left holding its cached value).
///
/// ## The two passes — and why order matters
///
/// We create **all** sheets before filling **any** cell. `spreadsheet-core`
/// resolves a cross-sheet reference (`Sheet2!A1`) through its own internal
/// `name → SheetId` map at `set_formula` time; that map must already contain
/// every sheet, or a formula that names a not-yet-created sheet would fail to
/// wire its dependency edge. Creating the sheets first makes the whole workbook
/// visible before any formula is parsed.
pub fn evaluate_workbook_verbose(sml: &SmlWorkbook) -> Result<Evaluation, EvalError> {
    let mut core = CoreWorkbook::new();

    // ---- Pass 1: create every sheet, in workbook order. ----------------
    // add_sheet returns a dense SheetId (0, 1, 2, …) that matches the M3
    // sheet order, so we can index core sheets by the same position; we keep
    // the returned ids aligned with `sml.sheets()`.
    let sheet_ids: Vec<_> = sml
        .sheets()
        .iter()
        .map(|sheet| core.add_sheet(sheet.name.clone()))
        .collect();

    // ---- Pass 2: fill every populated cell. ----------------------------
    let mut diagnostics = Vec::new();
    for (sheet, &sheet_id) in sml.sheets().iter().zip(&sheet_ids) {
        for cell in sheet.cells() {
            let addr = a1_to_core(&cell.reference)?;
            match &cell.formula {
                Some(text) => {
                    // Feed the raw <f> body to the engine's parser (leading `=`
                    // is optional). On a parse error, don't abort — fall back to
                    // the cached value as a literal and record a diagnostic.
                    if let Err(e) = core.set_formula(sheet_id, addr, text) {
                        core.set_value(sheet_id, addr, sml_value_to_core(&cell.value));
                        diagnostics.push(FormulaDiagnostic {
                            sheet: sheet.name.clone(),
                            reference: cell.reference.clone(),
                            formula: text.clone(),
                            message: e.to_string(),
                        });
                    }
                }
                None => {
                    core.set_value(sheet_id, addr, sml_value_to_core(&cell.value));
                }
            }
        }
    }

    // ---- Recompute. The engine topologically orders the formula cells,
    //      collapses cycles to #REF!, and caps oversized ranges (2^20). ---
    core.recalc_all();

    Ok(Evaluation {
        workbook: core,
        diagnostics,
    })
}

/// Convenience: open `.xlsx` bytes (M3) and immediately evaluate (M5).
pub fn open_and_evaluate(bytes: &[u8]) -> Result<CoreWorkbook, EvalError> {
    let sml = open_workbook(bytes)?;
    evaluate_workbook(&sml)
}

/// Read a *computed* value out of an engine workbook by (sheet name, A1). A
/// small ergonomic helper for tests and CLIs so callers don't have to resolve
/// the [`SheetId`](spreadsheet_core::SheetId) and parse the A1 themselves.
///
/// Returns `None` if the sheet name is unknown, the A1 is unparseable, or the
/// cell has never been touched.
pub fn computed_value(wb: &CoreWorkbook, sheet: &str, a1: &str) -> Option<CellValue> {
    let sheet_id = wb.sheet_id(sheet)?;
    let addr = a1_to_core(a1).ok()?;
    wb.get_value(sheet_id, addr)
}

/// Parse an M3-style A1 reference into a core [`CellAddress`].
///
/// We reuse M3's [`parse_a1_ref`] (which returns `(col, row)`, both 1-based)
/// rather than `spreadsheet-core`'s own parser, so both crates agree on exactly
/// which strings are valid refs — then feed the pair into
/// [`CellAddress::new`], **which takes `(row, col)`** (row first). Getting that
/// argument order right is the one subtle bit of the whole adapter.
fn a1_to_core(a1: &str) -> Result<CellAddress, EvalError> {
    let (col, row) = parse_a1_ref(a1).ok_or_else(|| EvalError::BadReference(a1.to_string()))?;
    Ok(CellAddress::new(row, col))
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_conversion_covers_every_variant() {
        assert_eq!(
            sml_value_to_core(&SmlValue::Number(3.5)),
            CellValue::Number(3.5)
        );
        assert_eq!(
            sml_value_to_core(&SmlValue::Text("hi".into())),
            CellValue::Text("hi".into())
        );
        assert_eq!(
            sml_value_to_core(&SmlValue::Bool(true)),
            CellValue::Boolean(true)
        );
        assert_eq!(
            sml_value_to_core(&SmlValue::Bool(false)),
            CellValue::Boolean(false)
        );
        assert_eq!(sml_value_to_core(&SmlValue::Empty), CellValue::Empty);
        assert_eq!(
            sml_value_to_core(&SmlValue::Error("#DIV/0!".into())),
            CellValue::Error(SpreadsheetError::DivZero)
        );
    }

    #[test]
    fn error_text_maps_every_code() {
        assert_eq!(parse_error_text("#DIV/0!"), SpreadsheetError::DivZero);
        assert_eq!(parse_error_text("#N/A"), SpreadsheetError::NotAvailable);
        assert_eq!(parse_error_text("#NAME?"), SpreadsheetError::Name);
        assert_eq!(parse_error_text("#NUM!"), SpreadsheetError::Num);
        assert_eq!(parse_error_text("#REF!"), SpreadsheetError::Ref);
        assert_eq!(parse_error_text("#VALUE!"), SpreadsheetError::Value);
        assert_eq!(parse_error_text("#NULL!"), SpreadsheetError::Null);
        // Whitespace is tolerated.
        assert_eq!(parse_error_text("  #N/A  "), SpreadsheetError::NotAvailable);
        // Unknown → the safe #VALUE! default.
        assert_eq!(parse_error_text("#WHAT?"), SpreadsheetError::Value);
    }

    #[test]
    fn a1_argument_order_is_row_then_col() {
        // B3 → col 2, row 3. CellAddress stores row=3, col=2.
        let addr = a1_to_core("B3").unwrap();
        assert_eq!(addr.row, 3);
        assert_eq!(addr.col, 2);
        // AA10 → col 27, row 10.
        let addr = a1_to_core("AA10").unwrap();
        assert_eq!(addr.row, 10);
        assert_eq!(addr.col, 27);
    }

    #[test]
    fn a1_rejects_garbage() {
        assert!(matches!(a1_to_core(""), Err(EvalError::BadReference(_))));
        assert!(matches!(a1_to_core("1A"), Err(EvalError::BadReference(_))));
    }

    #[test]
    fn eval_error_display_and_from() {
        let e: EvalError = XlsxError::MissingWorkbook.into();
        assert!(e.to_string().contains("could not open workbook"));
        let e = EvalError::BadReference("??".into());
        assert!(e.to_string().contains("??"));
    }
}
