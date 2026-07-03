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
use crate::fixture::{MINIMAL_XLSX, STYLED_XLSX};
use crate::styles::{
    classify_id, serial_to_date, serial_to_datetime, CellRange, NumberFormatKind, StyleTable,
};
use coding_adventures_xml_parser::parse_xml;

// ---------------------------------------------------------------------------
// Helper: parse an XML fragment and return its root element.
// ---------------------------------------------------------------------------

/// Wrap a `<c>`/`<si>` fragment in a namespaced root so the SpreadsheetML
/// namespace resolves, then return the requested first child.
fn parse_fragment(xml: &str) -> XmlElement {
    parse_xml(xml).expect("fragment parses").root
}

/// Decode a single `<c>` fragment (already the root element). Uses an empty
/// style table, so cells decode as in M3 (no number format) unless a test
/// supplies its own table via [`decode_c_styled`].
fn decode_c(xml: &str, shared: &[String]) -> Result<Cell, XlsxError> {
    decode_c_styled(xml, shared, &StyleTable::empty())
}

/// Decode a `<c>` fragment against a specific [`StyleTable`], so tests can drive
/// the `s=` → numFmtId → format resolution.
fn decode_c_styled(xml: &str, shared: &[String], styles: &StyleTable) -> Result<Cell, XlsxError> {
    let root = parse_fragment(xml);
    decode_cell(&root, shared, styles)
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

// ===========================================================================
// M4 — number formats, dates, merged cells, defined names
// ===========================================================================

/// The M4 end-to-end payoff: a styled workbook where a raw serial is a date, a
/// number is currency, and a fraction is a percentage.
#[test]
fn m4_end_to_end_styled_workbook() {
    let wb = open_workbook(STYLED_XLSX).expect("opens the styled fixture");
    assert_eq!(wb.sheet_names(), vec!["Report".to_string()]);
    let sheet = wb.sheet_by_name("Report").expect("Report sheet");

    // Headers are shared strings (unstyled → General → no number format).
    assert_eq!(sheet.cell("A1").unwrap().value, Value::Text("Date".into()));
    assert_eq!(sheet.cell("B1").unwrap().value, Value::Text("Amount".into()));
    assert!(sheet.cell("A1").unwrap().number_format.is_none());

    // A2 = 45292, styled numFmtId 14 (built-in date) → 2024-01-01.
    let a2 = sheet.cell("A2").unwrap();
    assert_eq!(a2.value, Value::Number(45292.0)); // raw value UNCHANGED (M3 compat)
    assert_eq!(a2.format_kind(), NumberFormatKind::Date);
    assert_eq!(a2.number_format.as_ref().unwrap().id, 14);
    assert_eq!(a2.as_date().as_deref(), Some("2024-01-01"));
    assert_eq!(a2.formatted(), "2024-01-01");

    // B2 = 1234.5, styled numFmtId 164 (custom currency "$"#,##0.00).
    let b2 = sheet.cell("B2").unwrap();
    assert_eq!(b2.value, Value::Number(1234.5)); // still a raw number
    assert_eq!(b2.format_kind(), NumberFormatKind::Currency);
    let fmt = b2.number_format.as_ref().unwrap();
    assert_eq!(fmt.id, 164);
    assert_eq!(fmt.code, "\"$\"#,##0.00");
    assert!(b2.as_date().is_none());

    // B4 = 0.25, styled numFmtId 10 (built-in 0.00% percent).
    let b4 = sheet.cell("B4").unwrap();
    assert_eq!(b4.value, Value::Number(0.25));
    assert_eq!(b4.format_kind(), NumberFormatKind::Percent);
    assert_eq!(b4.number_format.as_ref().unwrap().id, 10);
    assert_eq!(b4.formatted(), "25%");

    // A4 = "Rate" (shared string, unstyled).
    assert_eq!(sheet.cell("A4").unwrap().value, Value::Text("Rate".into()));

    // Merged range A1:B1.
    assert_eq!(
        sheet.merged_ranges(),
        &[CellRange {
            start: (1, 1),
            end: (2, 1)
        }]
    );

    // Defined name TaxRate → Report!$B$4.
    assert!(wb
        .defined_names()
        .iter()
        .any(|(n, r)| n == "TaxRate" && r == "Report!$B$4"));
}

/// The exact currency format code, checked verbatim (the `&quot;` entities in
/// the XML must be decoded to real quotes).
#[test]
fn m4_currency_format_code_decoded() {
    let wb = open_workbook(STYLED_XLSX).unwrap();
    let sheet = wb.sheet_by_name("Report").unwrap();
    let code = &sheet.cell("B2").unwrap().number_format.as_ref().unwrap().code;
    assert_eq!(code, "\"$\"#,##0.00");
}

// --- built-in id → kind mapping -------------------------------------------

#[test]
fn m4_builtin_id_classification() {
    assert_eq!(classify_id(0), NumberFormatKind::General);
    assert_eq!(classify_id(1), NumberFormatKind::Number); // "0"
    assert_eq!(classify_id(2), NumberFormatKind::Number); // "0.00"
    assert_eq!(classify_id(3), NumberFormatKind::Number); // "#,##0"
    assert_eq!(classify_id(9), NumberFormatKind::Percent); // "0%"
    assert_eq!(classify_id(10), NumberFormatKind::Percent); // "0.00%"
    assert_eq!(classify_id(14), NumberFormatKind::Date); // "m/d/yyyy"
    assert_eq!(classify_id(15), NumberFormatKind::Date); // "d-mmm-yy"
    assert_eq!(classify_id(17), NumberFormatKind::Date); // "mmm-yy"
    assert_eq!(classify_id(20), NumberFormatKind::Time); // "h:mm"
    assert_eq!(classify_id(21), NumberFormatKind::Time); // "h:mm:ss"
    assert_eq!(classify_id(22), NumberFormatKind::DateTime); // "m/d/yyyy h:mm"
    assert_eq!(classify_id(45), NumberFormatKind::Time); // "mm:ss"
    assert_eq!(classify_id(46), NumberFormatKind::Time); // "[h]:mm:ss"
    assert_eq!(classify_id(49), NumberFormatKind::Text); // "@"
}

#[test]
fn m4_unknown_builtin_id_is_general() {
    // 23-36 are reserved/locale with no portable code → General fallback.
    assert_eq!(classify_id(30), NumberFormatKind::General);
    // A custom id has no id-based meaning → Other (code drives the real answer).
    assert_eq!(classify_id(200), NumberFormatKind::Other);
}

// --- custom code classification -------------------------------------------

#[test]
fn m4_custom_code_classification() {
    use crate::classify_format_code as c;
    assert_eq!(c("General"), NumberFormatKind::General);
    assert_eq!(c("@"), NumberFormatKind::Text);
    assert_eq!(c("0.0%"), NumberFormatKind::Percent);
    assert_eq!(c("yyyy-mm-dd"), NumberFormatKind::Date);
    assert_eq!(c("h:mm:ss"), NumberFormatKind::Time);
    assert_eq!(c("m/d/yyyy h:mm"), NumberFormatKind::DateTime);
    assert_eq!(c("[$\u{20ac}-x]#,##0.00"), NumberFormatKind::Currency);
    assert_eq!(c("\"$\"#,##0.00"), NumberFormatKind::Currency);
    assert_eq!(c("0.00"), NumberFormatKind::Number);
    assert_eq!(c("#,##0"), NumberFormatKind::Number);
    // Scientific notation still has 0/# placeholders → Number.
    assert_eq!(c("0.00E+00"), NumberFormatKind::Number);
    // A pure literal with no placeholders/fields → Other.
    assert_eq!(c("\"n/a\""), NumberFormatKind::Other);
}

#[test]
fn m4_literal_text_does_not_trigger_date() {
    // A quoted literal "May" must NOT be read as a month field. The overall code
    // is still a plain number.
    assert_eq!(
        crate::classify_format_code("\"May\"0.00"),
        NumberFormatKind::Number
    );
    // A literal 'd' inside quotes is not a day.
    assert_eq!(
        crate::classify_format_code("\"days\"0"),
        NumberFormatKind::Number
    );
    // Escaped currency-looking char inside quotes should still flag currency
    // only via the real currency detection; a bare $ outside quotes is currency.
    assert_eq!(
        crate::classify_format_code("$#,##0"),
        NumberFormatKind::Currency
    );
}

#[test]
fn m4_elapsed_time_bracket_is_time() {
    assert_eq!(
        crate::classify_format_code("[h]:mm:ss"),
        NumberFormatKind::Time
    );
    // A [Red] colour directive must not be mistaken for a field.
    assert_eq!(
        crate::classify_format_code("[Red]0.00"),
        NumberFormatKind::Number
    );
}

// --- serial → date edge cases ---------------------------------------------

#[test]
fn m4_serial_to_date_edges() {
    assert_eq!(serial_to_date(1.0).as_deref(), Some("1900-01-01"));
    assert_eq!(serial_to_date(59.0).as_deref(), Some("1900-02-28"));
    // Serial 60 is Excel's phantom 1900-02-29 (the leap-year bug).
    assert_eq!(serial_to_date(60.0).as_deref(), Some("1900-02-29"));
    assert_eq!(serial_to_date(61.0).as_deref(), Some("1900-03-01"));
    assert_eq!(serial_to_date(45292.0).as_deref(), Some("2024-01-01"));
    // The classic Unix-epoch serial.
    assert_eq!(serial_to_date(25569.0).as_deref(), Some("1970-01-01"));
    // Fractional part is dropped by as_date.
    assert_eq!(serial_to_date(45292.75).as_deref(), Some("2024-01-01"));
}

#[test]
fn m4_serial_to_datetime() {
    assert_eq!(
        serial_to_datetime(45292.0).as_deref(),
        Some("2024-01-01T00:00:00")
    );
    assert_eq!(
        serial_to_datetime(45292.5).as_deref(),
        Some("2024-01-01T12:00:00")
    );
    // 0.25 of a day = 06:00.
    assert_eq!(
        serial_to_datetime(1.25).as_deref(),
        Some("1900-01-01T06:00:00")
    );
}

// --- style table resolution ------------------------------------------------

/// A stylesheet with the fixture's four cellXfs, used to drive `format_for`.
fn sample_styles() -> StyleTable {
    let root = parse_fragment(&format!(
        "<styleSheet {XMLNS}>\
           <numFmts count=\"1\"><numFmt numFmtId=\"164\" formatCode=\"0.00 &quot;kg&quot;\"/></numFmts>\
           <cellXfs count=\"4\">\
             <xf numFmtId=\"0\"/>\
             <xf numFmtId=\"14\"/>\
             <xf numFmtId=\"164\"/>\
             <xf numFmtId=\"10\"/>\
           </cellXfs>\
         </styleSheet>"
    ));
    StyleTable::from_root(&root)
}

#[test]
fn m4_no_style_index_is_general() {
    // s= absent → no format attached, and no date rendering.
    let styles = sample_styles();
    let cell = decode_c_styled(&format!("<c r=\"A1\" {XMLNS}><v>45292</v></c>"), &[], &styles)
        .unwrap();
    assert!(cell.number_format.is_none());
    assert_eq!(cell.format_kind(), NumberFormatKind::General);
    assert!(cell.as_date().is_none());
    assert_eq!(cell.formatted(), "45292");
}

#[test]
fn m4_general_style_index_is_none() {
    // s="0" resolves to General (id 0) → still no NumberFormat (M3 compat).
    let styles = sample_styles();
    let cell = decode_c_styled(
        &format!("<c r=\"A1\" s=\"0\" {XMLNS}><v>45292</v></c>"),
        &[],
        &styles,
    )
    .unwrap();
    assert!(cell.number_format.is_none());
}

#[test]
fn m4_date_style_resolves_and_renders() {
    let styles = sample_styles();
    let cell = decode_c_styled(
        &format!("<c r=\"A2\" s=\"1\" {XMLNS}><v>45292</v></c>"),
        &[],
        &styles,
    )
    .unwrap();
    assert_eq!(cell.format_kind(), NumberFormatKind::Date);
    assert_eq!(cell.as_date().as_deref(), Some("2024-01-01"));
}

#[test]
fn m4_custom_style_resolves() {
    // s="2" → numFmtId 164 → custom code 0.00 "kg" → a plain Number.
    let styles = sample_styles();
    let cell = decode_c_styled(
        &format!("<c r=\"A1\" s=\"2\" {XMLNS}><v>3.5</v></c>"),
        &[],
        &styles,
    )
    .unwrap();
    let fmt = cell.number_format.as_ref().unwrap();
    assert_eq!(fmt.id, 164);
    assert_eq!(fmt.code, "0.00 \"kg\"");
    assert_eq!(fmt.kind, NumberFormatKind::Number);
}

#[test]
fn m4_out_of_range_style_index_is_graceful() {
    // s="99" is out of range for a 4-entry cellXfs → no format, no panic.
    let styles = sample_styles();
    let cell = decode_c_styled(
        &format!("<c r=\"A1\" s=\"99\" {XMLNS}><v>1</v></c>"),
        &[],
        &styles,
    )
    .unwrap();
    assert!(cell.number_format.is_none());
    assert_eq!(cell.format_kind(), NumberFormatKind::General);
}

#[test]
fn m4_empty_style_table_attaches_nothing() {
    let styles = StyleTable::empty();
    let cell = decode_c_styled(
        &format!("<c r=\"A1\" s=\"3\" {XMLNS}><v>0.25</v></c>"),
        &[],
        &styles,
    )
    .unwrap();
    assert!(cell.number_format.is_none());
}

// --- CellRange parsing -----------------------------------------------------

#[test]
fn m4_cell_range_parse() {
    assert_eq!(
        CellRange::parse("A1:B1"),
        Some(CellRange {
            start: (1, 1),
            end: (2, 1)
        })
    );
    assert_eq!(
        CellRange::parse("B2:D10"),
        Some(CellRange {
            start: (2, 2),
            end: (4, 10)
        })
    );
    assert!(CellRange::parse("A1").is_none()); // no colon
    assert!(CellRange::parse("A1:").is_none()); // bad endpoint
    assert!(CellRange::parse(":B2").is_none());
}

// --- formatted() renderings ------------------------------------------------

#[test]
fn m4_formatted_variants() {
    // Text / bool / error / empty pass through their natural forms.
    let text = decode_c(&format!("<c r=\"A1\" t=\"str\" {XMLNS}><v>hi</v></c>"), &[]).unwrap();
    assert_eq!(text.formatted(), "hi");
    let b = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}><v>1</v></c>"), &[]).unwrap();
    assert_eq!(b.formatted(), "TRUE");
    let bf = decode_c(&format!("<c r=\"A1\" t=\"b\" {XMLNS}><v>0</v></c>"), &[]).unwrap();
    assert_eq!(bf.formatted(), "FALSE");
    let e = decode_c(&format!("<c r=\"A1\" t=\"e\" {XMLNS}><v>#N/A</v></c>"), &[]).unwrap();
    assert_eq!(e.formatted(), "#N/A");
    let empty = decode_c(&format!("<c r=\"A1\" {XMLNS}></c>"), &[]).unwrap();
    assert_eq!(empty.formatted(), "");
    // A plain number.
    let n = decode_c(&format!("<c r=\"A1\" {XMLNS}><v>12.5</v></c>"), &[]).unwrap();
    assert_eq!(n.formatted(), "12.5");
}

#[test]
fn m4_builtin_format_code_table() {
    assert_eq!(crate::builtin_format_code(14), Some("m/d/yyyy"));
    assert_eq!(crate::builtin_format_code(0), Some("General"));
    assert_eq!(crate::builtin_format_code(49), Some("@"));
    // Reserved id → None.
    assert_eq!(crate::builtin_format_code(30), None);
}

/// Exercise every entry of the built-in table so each match arm is hit and its
/// classification is sane (numbers stay numbers, accounting stays number-ish,
/// AM/PM times are times, fractions are Other/Number).
#[test]
fn m4_builtin_format_code_full_table() {
    use crate::classify_format_code as c;
    for id in 0u32..=49 {
        if let Some(code) = crate::builtin_format_code(id) {
            // Every built-in code must classify without panicking.
            let _ = c(code);
        }
    }
    // Spot-check the less-common arms explicitly.
    assert_eq!(crate::builtin_format_code(4), Some("#,##0.00"));
    assert_eq!(crate::builtin_format_code(11), Some("0.00E+00"));
    assert_eq!(crate::builtin_format_code(12), Some("# ?/?"));
    assert_eq!(crate::builtin_format_code(13), Some("# ??/??"));
    assert_eq!(crate::builtin_format_code(16), Some("d-mmm"));
    assert_eq!(crate::builtin_format_code(18), Some("h:mm AM/PM"));
    assert_eq!(crate::builtin_format_code(19), Some("h:mm:ss AM/PM"));
    assert_eq!(crate::builtin_format_code(37), Some("#,##0 ;(#,##0)"));
    assert_eq!(crate::builtin_format_code(38), Some("#,##0 ;[Red](#,##0)"));
    assert_eq!(crate::builtin_format_code(39), Some("#,##0.00;(#,##0.00)"));
    assert_eq!(
        crate::builtin_format_code(40),
        Some("#,##0.00;[Red](#,##0.00)")
    );
    assert_eq!(crate::builtin_format_code(47), Some("mmss.0"));
    assert_eq!(crate::builtin_format_code(48), Some("##0.0E+0"));
    // AM/PM formats are times; accounting formats are numbers.
    assert_eq!(classify_id(18), NumberFormatKind::Time);
    assert_eq!(classify_id(38), NumberFormatKind::Number);
}

/// The `@` text token and an escaped currency char (`\$`) outside quotes.
#[test]
fn m4_escaped_and_at_tokens() {
    // A bare @ mixed with other text is still Text.
    assert_eq!(
        crate::classify_format_code("\\ @"),
        NumberFormatKind::Text
    );
    // An escaped dollar sign flags currency.
    assert_eq!(
        crate::classify_format_code("\\$#,##0"),
        NumberFormatKind::Currency
    );
    // A pure escaped literal with no signal is Other.
    assert_eq!(crate::classify_format_code("\\x"), NumberFormatKind::Other);
}

/// The datetime rounding carry: a fraction that rounds up to a full day must
/// carry into the next date rather than print `24:00:00`.
#[test]
fn m4_datetime_rounding_carry() {
    // A fraction within half a second of a full day rounds to 86400 s → carry
    // into the next day at 00:00:00 (rather than an invalid 24:00:00).
    let s = serial_to_datetime(1.0 + 0.999_999).unwrap();
    assert_eq!(s, "1900-01-02T00:00:00");
}

/// The `StyleTable::from_root` path over a stylesheet with neither numFmts nor
/// cellXfs must yield an empty (but valid) table.
#[test]
fn m4_empty_stylesheet_parses() {
    let root = parse_fragment(&format!("<styleSheet {XMLNS}></styleSheet>"));
    let table = StyleTable::from_root(&root);
    assert!(table.format_for(Some(0)).is_none());
    assert!(table.format_for(Some(5)).is_none());
    assert!(table.format_for(None).is_none());
}
