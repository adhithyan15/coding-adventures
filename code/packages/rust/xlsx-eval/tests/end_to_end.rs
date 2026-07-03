//! End-to-end tests: open a real `.xlsx` byte array, evaluate its formulas
//! **from scratch**, and check the computed values — proving M5 recomputes
//! rather than trusting the cached `<v>` on disk.

use coding_adventures_xlsx_eval::{
    computed_value, evaluate_workbook, evaluate_workbook_verbose, open_and_evaluate,
};
use spreadsheet_core::{CellValue, Workbook as CoreWorkbook};

mod calc_xlsx;
mod minimal_xlsx;

use calc_xlsx::CALC_XLSX;
use minimal_xlsx::MINIMAL_XLSX;

// ---------------------------------------------------------------------------
// The headline: MINIMAL_XLSX — "Revenue" sheet, B1 = 1000, B2 = SUM(B1:B1).
// ---------------------------------------------------------------------------

#[test]
fn minimal_xlsx_recomputes_sum_from_scratch() {
    let wb = open_and_evaluate(MINIMAL_XLSX).expect("open + evaluate");

    // B2 = SUM(B1:B1) — the ENGINE computed this; we ignored the cached <v>.
    assert_eq!(
        computed_value(&wb, "Revenue", "B2"),
        Some(CellValue::Number(1000.0)),
        "B2 SUM(B1:B1) should recompute to 1000"
    );

    // The operand B1 is the literal 1000.
    assert_eq!(
        computed_value(&wb, "Revenue", "B1"),
        Some(CellValue::Number(1000.0)),
    );

    // A1 is a shared string "Q1" — must come through as Text.
    assert_eq!(
        computed_value(&wb, "Revenue", "A1"),
        Some(CellValue::Text("Q1".into())),
    );
}

#[test]
fn minimal_xlsx_has_no_formula_diagnostics() {
    let sml = coding_adventures_spreadsheetml::open_workbook(MINIMAL_XLSX).unwrap();
    let eval = evaluate_workbook_verbose(&sml).unwrap();
    assert!(
        eval.diagnostics.is_empty(),
        "clean workbook should have no formula diagnostics, got {:?}",
        eval.diagnostics
    );
}

// ---------------------------------------------------------------------------
// CALC_XLSX — a hand-built book with a multi-cell SUM and an arithmetic
// formula, both with DELIBERATELY WRONG cached values, to prove recompute.
// ---------------------------------------------------------------------------

#[test]
fn calc_xlsx_recomputes_multi_cell_sum_and_arithmetic() {
    let wb = open_and_evaluate(CALC_XLSX).expect("open + evaluate");

    // Literals.
    assert_eq!(computed_value(&wb, "Calc", "A1"), Some(CellValue::Number(10.0)));
    assert_eq!(computed_value(&wb, "Calc", "A2"), Some(CellValue::Number(20.0)));
    assert_eq!(computed_value(&wb, "Calc", "A3"), Some(CellValue::Number(30.0)));

    // B1 = SUM(A1:A3): cached on disk as the WRONG 999; engine recomputes 60.
    assert_eq!(
        computed_value(&wb, "Calc", "B1"),
        Some(CellValue::Number(60.0)),
        "SUM(A1:A3) must recompute to 60, not the stale cached 999"
    );

    // B2 = A1*2: cached on disk as the WRONG 0; engine recomputes 20.
    assert_eq!(
        computed_value(&wb, "Calc", "B2"),
        Some(CellValue::Number(20.0)),
        "A1*2 must recompute to 20, not the stale cached 0"
    );
}

// ---------------------------------------------------------------------------
// Graceful degradation: a garbage formula falls back to its cached value and
// does not panic or poison the rest of the workbook.
// ---------------------------------------------------------------------------

#[test]
fn bad_formula_falls_back_to_cached_value_and_records_diagnostic() {
    // The M3 model's fields are private, so we build a tiny .xlsx in-memory
    // with a malformed <f> and drive it through the public open + evaluate path.
    // Sheet "S": A1 = 5, A2 = formula ")(" (garbage) cached 42.
    let bytes = build_xlsx(
        "S",
        r#"<row r="1"><c r="A1"><v>5</v></c></row>"#.to_string()
            + r#"<row r="2"><c r="A2"><f>)(</f><v>42</v></c></row>"#,
    );

    let sml = coding_adventures_spreadsheetml::open_workbook(&bytes).unwrap();
    let eval = evaluate_workbook_verbose(&sml).expect("evaluate must not abort");

    // The good cell still computed.
    assert_eq!(
        computed_value(&eval.workbook, "S", "A1"),
        Some(CellValue::Number(5.0))
    );
    // The bad formula fell back to its cached value 42 — no panic, still usable.
    assert_eq!(
        computed_value(&eval.workbook, "S", "A2"),
        Some(CellValue::Number(42.0))
    );
    // …and it was recorded as a diagnostic.
    assert_eq!(eval.diagnostics.len(), 1);
    let d = &eval.diagnostics[0];
    assert_eq!(d.sheet, "S");
    assert_eq!(d.reference, "A2");
    assert_eq!(d.formula, ")(");
    assert!(!d.message.is_empty());
}

// ---------------------------------------------------------------------------
// An empty workbook (no sheets, no cells) evaluates to an empty engine book.
// ---------------------------------------------------------------------------

#[test]
fn empty_workbook_evaluates_cleanly() {
    let bytes = build_workbook_no_sheets();
    let sml = coding_adventures_spreadsheetml::open_workbook(&bytes).unwrap();
    let wb = evaluate_workbook(&sml).expect("empty evaluate");
    assert_eq!(wb.sheet_count(), 0);
    assert_eq!(computed_value(&wb, "Nope", "A1"), None);
}

#[test]
fn open_and_evaluate_reports_open_error_on_garbage_bytes() {
    // (CoreWorkbook isn't Debug, so we can't `.unwrap_err()`; match instead.)
    match open_and_evaluate(b"not a zip at all") {
        Ok(_) => panic!("garbage bytes should not open"),
        Err(e) => assert!(e.to_string().contains("could not open workbook")),
    }
}

// A single-sheet book with cells is fully populated after evaluate.
#[test]
fn returned_workbook_is_a_usable_core_workbook() {
    let wb: CoreWorkbook = open_and_evaluate(MINIMAL_XLSX).unwrap();
    let sheet_id = wb.sheet_id("Revenue").expect("Revenue sheet exists");
    // used_range spans the populated cells.
    assert!(wb.used_range(sheet_id).is_some());
}

// ===========================================================================
// Test helpers — build minimal .xlsx byte arrays in-memory (no zip crate dep;
// we assemble stored (uncompressed) ZIP entries by hand).
// ===========================================================================

/// Build a one-sheet `.xlsx` whose `<sheetData>` is the given inner XML.
fn build_xlsx(sheet_name: &str, sheet_data_rows: String) -> Vec<u8> {
    let sheet_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{sheet_data_rows}</sheetData></worksheet>"#
    );
    let workbook_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="{sheet_name}" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    assemble_package(&workbook_xml, Some(&sheet_xml))
}

/// Build a workbook with an empty `<sheets/>` — no worksheets at all.
fn build_workbook_no_sheets() -> Vec<u8> {
    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets/></workbook>"#;
    assemble_package(workbook_xml, None)
}

fn assemble_package(workbook_xml: &str, sheet_xml: Option<&str>) -> Vec<u8> {
    let mut sheet_override = String::new();
    let mut wb_rels_entries = String::new();
    if sheet_xml.is_some() {
        sheet_override = r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#.to_string();
        wb_rels_entries = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>"#.to_string();
    }

    let content_types = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>{sheet_override}</Types>"#
    );
    let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    let wb_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{wb_rels_entries}</Relationships>"#
    );

    let mut parts: Vec<(&str, Vec<u8>)> = vec![
        ("[Content_Types].xml", content_types.into_bytes()),
        ("_rels/.rels", root_rels.as_bytes().to_vec()),
        ("xl/workbook.xml", workbook_xml.as_bytes().to_vec()),
        ("xl/_rels/workbook.xml.rels", wb_rels.into_bytes()),
    ];
    if let Some(s) = sheet_xml {
        parts.push(("xl/worksheets/sheet1.xml", s.as_bytes().to_vec()));
    }
    zip_stored(&parts)
}

/// Assemble a ZIP archive using only STORED (method 0, uncompressed) entries —
/// enough for the M3 reader to open. Hand-rolled so this test needs no zip
/// crate. CRC-32 (IEEE) is computed per entry.
fn zip_stored(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    let mut out = Vec::new();
    let mut central = Vec::new();
    let mut offsets = Vec::new();

    for (name, data) in parts {
        let offset = out.len() as u32;
        offsets.push(offset);
        let crc = crc32(data);
        let name_bytes = name.as_bytes();
        // Local file header.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes()); // signature
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method = stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
    }

    let central_start = out.len() as u32;
    for ((name, data), &offset) in parts.iter().zip(&offsets) {
        let crc = crc32(data);
        let name_bytes = name.as_bytes();
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes()); // signature
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&0u16.to_le_bytes()); // method
        central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        central.extend_from_slice(&0u16.to_le_bytes()); // mod date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let central_size = central.len() as u32;
    out.extend_from_slice(&central);

    // End of central directory.
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // disk
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with central
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}
