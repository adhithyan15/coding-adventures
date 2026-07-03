//! Tests for the SpreadsheetML reader.
//!
//! Two layers of tests:
//!
//! * **End-to-end** — open the real `.xlsx` fixture bytes and assert the whole
//!   Revenue-sheet grid, exercising the OPC package, shared-string, and r:id
//!   indirections together (the M3 payoff).
//! * **Unit** — the pure helpers (`parse_a1_ref`) and the cell decoder
//!   (`decode_cell`), driven by hand-written XML fragments so we can cover every
//!   `t` variant (inline string, boolean, error, empty, rich text) without
//!   crafting a whole ZIP for each.

use super::*;
use crate::fixture::MINIMAL_XLSX;
use coding_adventures_xml_parser::parse_xml;

// ---------------------------------------------------------------------------
// Helper: parse an XML fragment and return its root element.
// ---------------------------------------------------------------------------

/// Wrap a `<c>`/`<si>` fragment in a namespaced root so the SpreadsheetML
/// namespace resolves, then return the requested first child.
fn parse_fragment(xml: &str) -> XmlElement {
    parse_xml(xml).expect("fragment parses").root
}

/// Decode a single `<c>` fragment (already the root element).
fn decode_c(xml: &str, shared: &[String]) -> Result<Cell, XlsxError> {
    let root = parse_fragment(xml);
    decode_cell(&root, shared)
}

const XMLNS: &str = "xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"";

// ===========================================================================
// End-to-end payoff
// ===========================================================================

#[test]
fn end_to_end_revenue_sheet() {
    let wb = open_workbook(MINIMAL_XLSX).expect("opens the fixture");

    // Exactly one sheet, named "Revenue".
    assert_eq!(wb.sheet_names(), vec!["Revenue".to_string()]);

    let sheet = wb.sheet_by_name("Revenue").expect("Revenue sheet exists");

    // A1 = shared string "Q1"; B1 = number 1000.
    assert_eq!(sheet.cell("A1").unwrap().value, Value::Text("Q1".into()));
    assert_eq!(sheet.cell("B1").unwrap().value, Value::Number(1000.0));

    // A2 = shared string "Total".
    assert_eq!(sheet.cell("A2").unwrap().value, Value::Text("Total".into()));

    // B2 = formula SUM(B1:B1), cached value 1000.
    let b2 = sheet.cell("B2").unwrap();
    assert_eq!(b2.formula, Some("SUM(B1:B1)".to_string()));
    assert_eq!(b2.value, Value::Number(1000.0));
}

#[test]
fn end_to_end_cells_iterate_in_reading_order() {
    let wb = open_workbook(MINIMAL_XLSX).unwrap();
    let sheet = wb.sheet_by_name("Revenue").unwrap();
    let refs: Vec<&str> = sheet.cells().map(|c| c.reference.as_str()).collect();
    // Row-major, left-to-right.
    assert_eq!(refs, vec!["A1", "B1", "A2", "B2"]);
    assert_eq!(sheet.cell_count(), 4);
}

#[test]
fn end_to_end_sheets_accessor() {
    let wb = open_workbook(MINIMAL_XLSX).unwrap();
    assert_eq!(wb.sheets().len(), 1);
    assert!(wb.sheet_by_name("Missing").is_none());
}

// ===========================================================================
// A1 reference parsing
// ===========================================================================

#[test]
fn a1_ref_basic() {
    assert_eq!(parse_a1_ref("A1"), Some((1, 1)));
    assert_eq!(parse_a1_ref("B2"), Some((2, 2)));
    assert_eq!(parse_a1_ref("Z1"), Some((26, 1)));
    assert_eq!(parse_a1_ref("AA10"), Some((27, 10)));
}

#[test]
fn a1_ref_bijective_base26() {
    // AA = 26*1 + 1 = 27, AB = 28, AZ = 52, BA = 53.
    assert_eq!(parse_a1_ref("AB1").unwrap().0, 28);
    assert_eq!(parse_a1_ref("AZ1").unwrap().0, 52);
    assert_eq!(parse_a1_ref("BA1").unwrap().0, 53);
    // Multi-row.
    assert_eq!(parse_a1_ref("C123"), Some((3, 123)));
}

#[test]
fn a1_ref_lowercase_tolerated() {
    // Producers are uppercase, but be forgiving.
    assert_eq!(parse_a1_ref("b2"), Some((2, 2)));
}

#[test]
fn a1_ref_rejects_malformed() {
    assert_eq!(parse_a1_ref(""), None); // empty
    assert_eq!(parse_a1_ref("A"), None); // no row
    assert_eq!(parse_a1_ref("1"), None); // no column
    assert_eq!(parse_a1_ref("1A"), None); // digits first
    assert_eq!(parse_a1_ref("A1B"), None); // trailing junk
    assert_eq!(parse_a1_ref("A0"), None); // row 0
    assert_eq!(parse_a1_ref("$A$1"), None); // absolute refs not accepted here
    assert_eq!(parse_a1_ref("A 1"), None); // space
}

// ===========================================================================
// Cell decoding — one test per `t` variant
// ===========================================================================

#[test]
fn decode_number_no_type() {
    let cell = decode_c(&format!("<c r=\"B1\" {XMLNS}><v>1000</v></c>"), &[]).unwrap();
    assert_eq!(cell.reference, "B1");
    assert_eq!(cell.value, Value::Number(1000.0));
    assert!(cell.formula.is_none());
}

#[test]
fn decode_number_explicit_type_n() {
    let cell = decode_c(&format!("<c r=\"B1\" t=\"n\" {XMLNS}><v>3.5</v></c>"), &[]).unwrap();
    assert_eq!(cell.value, Value::Number(3.5));
}

#[test]
fn decode_shared_string() {
    let shared = vec!["Q1".to_string(), "Total".to_string()];
    let cell = decode_c(&format!("<c r=\"A1\" t=\"s\" {XMLNS}><v>0</v></c>"), &shared).unwrap();
    assert_eq!(cell.value, Value::Text("Q1".into()));
    let cell = decode_c(&format!("<c r=\"A2\" t=\"s\" {XMLNS}><v>1</v></c>"), &shared).unwrap();
    assert_eq!(cell.value, Value::Text("Total".into()));
}

#[test]
fn decode_shared_string_out_of_range() {
    let shared = vec!["only".to_string()];
    let err = decode_c(&format!("<c r=\"A1\" t=\"s\" {XMLNS}><v>5</v></c>"), &shared).unwrap_err();
    assert_eq!(err, XlsxError::BadSharedStringIndex(5));
}

#[test]
fn decode_shared_string_bad_index_text() {
    let shared = vec!["only".to_string()];
    let err =
        decode_c(&format!("<c r=\"A1\" t=\"s\" {XMLNS}><v>abc</v></c>"), &shared).unwrap_err();
    assert!(matches!(err, XlsxError::BadSharedStringIndex(_)));
}

#[test]
fn decode_formula_str_result() {
    let cell = decode_c(
        &format!("<c r=\"A1\" t=\"str\" {XMLNS}><f>CONCAT(\"a\",\"b\")</f><v>ab</v></c>"),
        &[],
    )
    .unwrap();
    assert_eq!(cell.value, Value::Text("ab".into()));
    assert_eq!(cell.formula, Some("CONCAT(\"a\",\"b\")".into()));
}

#[test]
fn decode_inline_string() {
    let cell = decode_c(
        &format!("<c r=\"A1\" t=\"inlineStr\" {XMLNS}><is><t>hello</t></is></c>"),
        &[],
    )
    .unwrap();
    assert_eq!(cell.value, Value::Text("hello".into()));
}

#[test]
fn decode_inline_string_rich_runs() {
    // Rich inline string: multiple <r><t> runs concatenate.
    let cell = decode_c(
        &format!(
            "<c r=\"A1\" t=\"inlineStr\" {XMLNS}><is><r><t>Hello, </t></r><r><t>World</t></r></is></c>"
        ),
        &[],
    )
    .unwrap();
    assert_eq!(cell.value, Value::Text("Hello, World".into()));
}

#[test]
fn decode_boolean() {
    let t = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}><v>1</v></c>"), &[]).unwrap();
    assert_eq!(t.value, Value::Bool(true));
    let f = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}><v>0</v></c>"), &[]).unwrap();
    assert_eq!(f.value, Value::Bool(false));
}

#[test]
fn decode_boolean_edge_cases() {
    // Absent <v> under t="b" → Empty.
    let empty = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}></c>"), &[]).unwrap();
    assert_eq!(empty.value, Value::Empty);
    // Non-standard truthy value.
    let truthy = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}><v>2</v></c>"), &[]).unwrap();
    assert_eq!(truthy.value, Value::Bool(true));
}

#[test]
fn decode_error() {
    let cell = decode_c(&format!("<c r=\"A1\" t=\"e\" {XMLNS}><v>#DIV/0!</v></c>"), &[]).unwrap();
    assert_eq!(cell.value, Value::Error("#DIV/0!".into()));
}

#[test]
fn decode_error_absent_v() {
    let cell = decode_c(&format!("<c r=\"A1\" t=\"e\" {XMLNS}></c>"), &[]).unwrap();
    assert_eq!(cell.value, Value::Empty);
}

#[test]
fn decode_empty_cell() {
    let cell = decode_c(&format!("<c r=\"A1\" {XMLNS}></c>"), &[]).unwrap();
    assert_eq!(cell.value, Value::Empty);
    assert!(cell.formula.is_none());
}

#[test]
fn decode_formula_number_cached() {
    // A formula cell with a numeric cached value keeps both.
    let cell = decode_c(
        &format!("<c r=\"B2\" {XMLNS}><f>SUM(B1:B1)</f><v>1000</v></c>"),
        &[],
    )
    .unwrap();
    assert_eq!(cell.formula, Some("SUM(B1:B1)".into()));
    assert_eq!(cell.value, Value::Number(1000.0));
}

#[test]
fn decode_bad_number_errors() {
    let err = decode_c(&format!("<c r=\"A1\" {XMLNS}><v>not-a-number</v></c>"), &[]).unwrap_err();
    assert!(matches!(err, XlsxError::MalformedXml(_)));
}

// ===========================================================================
// Shared-string table parsing — single <t> vs rich runs
// ===========================================================================

#[test]
fn shared_string_rich_text_concatenation() {
    // <si> string value is the concatenation of all descendant <t> text.
    let sst = parse_fragment(&format!(
        "<sst {XMLNS}>\
           <si><t>Simple</t></si>\
           <si><r><t>Rich </t></r><r><t>text </t></r><r><t>here</t></r></si>\
         </sst>"
    ));
    let items: Vec<String> = sst
        .get_children(Some(SML_NS), "si")
        .iter()
        .map(|si| si.text_content())
        .collect();
    assert_eq!(items, vec!["Simple".to_string(), "Rich text here".to_string()]);
}

// ===========================================================================
// Error paths at the package level
// ===========================================================================

#[test]
fn open_non_xlsx_bytes_errors() {
    // Random bytes are not a ZIP → an OPC error, wrapped.
    let err = open_workbook(b"this is not a zip file at all").unwrap_err();
    assert!(matches!(err, XlsxError::Opc(_)));
}

#[test]
fn open_empty_bytes_errors() {
    let err = open_workbook(b"").unwrap_err();
    assert!(matches!(err, XlsxError::Opc(_)));
}

#[test]
fn error_display_is_readable() {
    // Exercise the Display impls so they are covered and non-empty.
    assert!(!XlsxError::MissingWorkbook.to_string().is_empty());
    assert!(!XlsxError::NotUtf8("/x".into()).to_string().is_empty());
    assert!(!XlsxError::MalformedXml("m".into()).to_string().is_empty());
    assert!(!XlsxError::MissingSheetPart("rId9".into())
        .to_string()
        .is_empty());
    assert!(!XlsxError::BadSharedStringIndex(3).to_string().is_empty());
    // Opc variant Display goes through the wrapped error.
    let opc = XlsxError::Opc(coding_adventures_opc::OpcError::MissingContentTypes);
    assert!(opc.to_string().contains("package error"));
}

// ===========================================================================
// A workbook with no shared strings part
// ===========================================================================

// Building a whole ZIP in-memory for this would duplicate the OPC crate's
// machinery. Instead we assert the *table-loading* contract directly: an empty
// table is the legal result when no sharedStrings relationship exists. The
// fixture's own numeric cells (B1, B2) already prove numbers decode without any
// shared-string lookup; here we prove a t="s" cell against an EMPTY table is the
// out-of-range error, which is exactly what a missing table would produce for a
// (malformed) text cell.
#[test]
fn no_shared_strings_table_is_empty_by_contract() {
    let empty_table: Vec<String> = Vec::new();
    // A numeric cell needs no table at all.
    let n = decode_c(&format!("<c r=\"B1\" {XMLNS}><v>42</v></c>"), &empty_table).unwrap();
    assert_eq!(n.value, Value::Number(42.0));
    // A t="s" cell against an empty table is out-of-range (index 0 missing).
    let err = decode_c(&format!("<c r=\"A1\" t=\"s\" {XMLNS}><v>0</v></c>"), &empty_table)
        .unwrap_err();
    assert_eq!(err, XlsxError::BadSharedStringIndex(0));
}
