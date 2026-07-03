//! read-xlsx — render a real `.xlsx` as an evaluated cell grid.
//!
//! This is the runnable end-goal of the OOXML effort: it stitches together the
//! entire zero-third-party-dependency stack to turn spreadsheet bytes into a
//! human-readable table of *computed* values.
//!
//! ```text
//! .xlsx bytes
//!   → zip           (unzip the OPC package; DEFLATE inflate)
//!   → xml-parser    (namespace-aware parse of each part)
//!   → opc           ([Content_Types].xml + .rels → resolve parts)
//!   → spreadsheetml (workbook → sheets-by-r:id → cells + shared strings)
//!   → styles.xml    (number formats: serial 45292 → "2024-01-01", %, currency)
//!   → xlsx-eval     (recompute <f> formulas via spreadsheet-core)
//! ```
//!
//! The library exposes [`render_xlsx`] (bytes → a printable report). The binary
//! (`src/main.rs`) is a thin wrapper: `read-xlsx <file.xlsx>` or `read-xlsx --demo`.

use coding_adventures_spreadsheetml::{self as sml, NumberFormatKind};
use coding_adventures_xlsx_eval::{computed_value, open_and_evaluate};

pub mod fixtures;

/// Errors that can arise while rendering a workbook.
#[derive(Debug)]
pub enum RenderError {
    /// The bytes could not be parsed as an .xlsx workbook.
    Open(String),
    /// The workbook parsed but its formulas could not be evaluated.
    Evaluate(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Open(m) => write!(f, "could not open .xlsx: {m}"),
            RenderError::Evaluate(m) => write!(f, "could not evaluate formulas: {m}"),
        }
    }
}
impl std::error::Error for RenderError {}

/// One row of the rendered report — public so callers can format it their own way.
#[derive(Debug, Clone)]
pub struct RenderedCell {
    pub reference: String,
    /// The value as a user would see it (number formats applied: dates, %, etc.).
    pub display: String,
    /// The `<f>` formula text, if the cell has one.
    pub formula: Option<String>,
    /// The value the formula engine *recomputed* from scratch (cached `<v>` ignored).
    pub computed: Option<String>,
    /// The classification of the cell's number format.
    pub kind: NumberFormatKind,
}

/// A rendered sheet: its name and its populated cells in reading order.
#[derive(Debug, Clone)]
pub struct RenderedSheet {
    pub name: String,
    pub cells: Vec<RenderedCell>,
}

/// Parse and evaluate `.xlsx` bytes into a structured, printable report.
///
/// Every formula cell is **recomputed** by the engine — the value on disk is
/// ignored — so this reflects what a live spreadsheet host would show.
pub fn render_xlsx(bytes: &[u8]) -> Result<Vec<RenderedSheet>, RenderError> {
    let book = sml::open_workbook(bytes).map_err(|e| RenderError::Open(format!("{e:?}")))?;
    let core = open_and_evaluate(bytes).map_err(|e| RenderError::Evaluate(format!("{e:?}")))?;

    let mut sheets = Vec::new();
    for sheet in book.sheets() {
        let mut cells = Vec::new();
        for cell in sheet.cells() {
            let computed = computed_value(&core, &sheet.name, &cell.reference)
                .map(|v| format!("{v:?}"));
            cells.push(RenderedCell {
                reference: cell.reference.clone(),
                display: cell.formatted(),
                formula: cell.formula.clone(),
                computed,
                kind: cell.format_kind(),
            });
        }
        sheets.push(RenderedSheet { name: sheet.name.clone(), cells });
    }
    Ok(sheets)
}

/// Format a report as a plain-text table (what the CLI prints).
pub fn format_report(sheets: &[RenderedSheet]) -> String {
    let mut out = String::new();
    for sheet in sheets {
        out.push_str(&format!("Sheet \"{}\" — {} cells\n", sheet.name, sheet.cells.len()));
        out.push_str(&format!(
            "  {:<5} {:<12} {:<20} {:<18} {}\n",
            "cell", "kind", "display", "formula", "recomputed"
        ));
        for c in &sheet.cells {
            out.push_str(&format!(
                "  {:<5} {:<12} {:<20} {:<18} {}\n",
                c.reference,
                format!("{:?}", c.kind),
                c.display,
                c.formula.clone().unwrap_or_else(|| "—".into()),
                c.computed.clone().unwrap_or_else(|| "—".into()),
            ));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_workbook_recomputes_sum() {
        let sheets = render_xlsx(fixtures::MINIMAL_XLSX).expect("render minimal");
        assert_eq!(sheets.len(), 1);
        let s = &sheets[0];
        assert_eq!(s.name, "Revenue");
        let b2 = s.cells.iter().find(|c| c.reference == "B2").expect("B2");
        assert_eq!(b2.formula.as_deref(), Some("SUM(B1:B1)"));
        // The engine recomputed the SUM from scratch.
        assert_eq!(b2.computed.as_deref(), Some("Number(1000.0)"));
        let a1 = s.cells.iter().find(|c| c.reference == "A1").expect("A1");
        assert_eq!(a1.display, "Q1"); // shared string
    }

    #[test]
    fn styled_workbook_formats_date_and_percent() {
        let sheets = render_xlsx(fixtures::STYLED_XLSX).expect("render styled");
        let s = &sheets[0];
        assert_eq!(s.name, "Report");
        let a2 = s.cells.iter().find(|c| c.reference == "A2").expect("A2");
        assert_eq!(a2.kind, NumberFormatKind::Date);
        assert_eq!(a2.display, "2024-01-01"); // serial 45292 rendered as a date
        let b4 = s.cells.iter().find(|c| c.reference == "B4").expect("B4");
        assert_eq!(b4.kind, NumberFormatKind::Percent);
    }

    #[test]
    fn format_report_mentions_sheet_and_cells() {
        let sheets = render_xlsx(fixtures::MINIMAL_XLSX).unwrap();
        let report = format_report(&sheets);
        assert!(report.contains("Sheet \"Revenue\""));
        assert!(report.contains("SUM(B1:B1)"));
        assert!(report.contains("recomputed"));
    }

    #[test]
    fn garbage_bytes_are_a_clean_error_not_a_panic() {
        let err = render_xlsx(b"not a zip").unwrap_err();
        assert!(matches!(err, RenderError::Open(_)));
        // Display impl works (no panic).
        let _ = format!("{err}");
    }
}
