//! Focused unit tests for `xlsx-writer` internals: A1 generation, number
//! formatting, the shared-string table, and XML shape. The end-to-end
//! round-trip through the repo's own readers lives in `tests/round_trip.rs`.

use super::*;

// ── A1 reference generation ───────────────────────────────────────────────

#[test]
fn col_to_letters_bijective_base26() {
    assert_eq!(col_to_letters(1), "A");
    assert_eq!(col_to_letters(26), "Z");
    assert_eq!(col_to_letters(27), "AA");
    assert_eq!(col_to_letters(28), "AB");
    assert_eq!(col_to_letters(52), "AZ");
    assert_eq!(col_to_letters(53), "BA");
    assert_eq!(col_to_letters(702), "ZZ");
    assert_eq!(col_to_letters(703), "AAA");
}

#[test]
fn a1_generation_round_trips_with_parse() {
    // Generating an A1 and parsing it back must be an identity on (col, row).
    for &(col, row) in &[(1u32, 1u32), (26, 5), (27, 100), (702, 2), (703, 1)] {
        let a1 = a1_of(col, row);
        assert_eq!(parse_a1(&a1), Some((col, row)), "round-trip {a1}");
    }
}

#[test]
fn parse_a1_rejects_garbage() {
    assert_eq!(parse_a1(""), None);
    assert_eq!(parse_a1("1A"), None);
    assert_eq!(parse_a1("A"), None);
    assert_eq!(parse_a1("A0"), None);
    assert_eq!(parse_a1("A1B"), None);
}

// ── Number formatting ─────────────────────────────────────────────────────

#[test]
fn numbers_format_without_trailing_zero() {
    assert_eq!(format_number(1000.0), "1000");
    assert_eq!(format_number(0.0), "0");
    assert_eq!(format_number(-42.0), "-42");
    assert_eq!(format_number(12.5), "12.5");
    assert_eq!(format_number(3.25), "3.25");
}

#[test]
fn non_finite_numbers_are_safe() {
    // NaN / inf are not representable in <v>; we emit "0" rather than bad XML.
    assert_eq!(format_number(f64::NAN), "0");
    assert_eq!(format_number(f64::INFINITY), "0");
    assert_eq!(format_number(f64::NEG_INFINITY), "0");
}

// ── Shared-string table ───────────────────────────────────────────────────

#[test]
fn shared_strings_dedup_and_index() {
    let mut sst = SharedStrings::default();
    assert_eq!(sst.intern("Q1"), 0);
    assert_eq!(sst.intern("Total"), 1);
    assert_eq!(sst.intern("Q1"), 0); // repeat reuses index 0
    assert_eq!(sst.intern("Total"), 1); // repeat reuses index 1
    assert_eq!(sst.intern("New"), 2);

    // 5 references, 3 distinct strings.
    assert_eq!(sst.total_refs, 5);
    assert_eq!(sst.strings.len(), 3);

    let xml = String::from_utf8(sst.to_xml()).unwrap();
    assert!(xml.contains("count=\"5\""));
    assert!(xml.contains("uniqueCount=\"3\""));
    assert!(xml.contains("<t xml:space=\"preserve\">Q1</t>"));
}

// ── XML escaping in cells and sheet names ─────────────────────────────────

#[test]
fn string_cells_are_xml_escaped() {
    let mut sst = SharedStrings::default();
    sst.intern("a & b < c > d \" e");
    let xml = String::from_utf8(sst.to_xml()).unwrap();
    assert!(xml.contains("a &amp; b &lt; c &gt; d &quot; e"));
    // The raw special chars must NOT survive unescaped in text.
    assert!(!xml.contains("a & b"));
}

#[test]
fn sheet_names_are_xml_escaped() {
    let sheets = vec![Sheet::new("Q1 & Q2 <report>")];
    let xml = String::from_utf8(workbook_xml(&sheets)).unwrap();
    assert!(xml.contains("name=\"Q1 &amp; Q2 &lt;report&gt;\""));
}

#[test]
fn formula_text_is_xml_escaped() {
    let mut sheet = Sheet::new("S");
    // A formula with a comparison operator contains '<' and '>'.
    sheet.set_formula("A1", "IF(B1<C1,1,0)", 0.0);
    let mut sst = SharedStrings::default();
    let xml = String::from_utf8(worksheet_xml(&sheet, &mut sst)).unwrap();
    assert!(xml.contains("<f>IF(B1&lt;C1,1,0)</f>"));
}

// ── Worksheet XML shape ───────────────────────────────────────────────────

#[test]
fn worksheet_groups_cells_into_rows() {
    let mut sheet = Sheet::new("S");
    sheet.set_string("A1", "x");
    sheet.set_number("B1", 5.0);
    sheet.set_number("A2", 7.0);
    let mut sst = SharedStrings::default();
    let xml = String::from_utf8(worksheet_xml(&sheet, &mut sst)).unwrap();

    // Two rows, with r="1" containing A1 and B1, r="2" containing A2.
    assert!(xml.contains("<row r=\"1\">"));
    assert!(xml.contains("<row r=\"2\">"));
    assert!(xml.contains("<c r=\"A1\" t=\"s\"><v>0</v></c>"));
    assert!(xml.contains("<c r=\"B1\"><v>5</v></c>"));
    assert!(xml.contains("<c r=\"A2\"><v>7</v></c>"));
    // Exactly two <row> opens and two closes → balanced.
    assert_eq!(xml.matches("<row ").count(), 2);
    assert_eq!(xml.matches("</row>").count(), 2);
}

#[test]
fn bad_ref_is_silent_noop() {
    let mut sheet = Sheet::new("S");
    sheet.set_number("not-a-ref", 1.0);
    sheet.set_string("", "x");
    assert!(sheet.cells.is_empty());
}

// ── Multiple sheets ───────────────────────────────────────────────────────

#[test]
fn multiple_sheets_get_sequential_rids() {
    let sheets = vec![Sheet::new("One"), Sheet::new("Two"), Sheet::new("Three")];
    let xml = String::from_utf8(workbook_xml(&sheets)).unwrap();
    assert!(xml.contains("name=\"One\" sheetId=\"1\" r:id=\"rId1\""));
    assert!(xml.contains("name=\"Two\" sheetId=\"2\" r:id=\"rId2\""));
    assert!(xml.contains("name=\"Three\" sheetId=\"3\" r:id=\"rId3\""));
}

// ── Empty workbook / empty sheet don't panic ──────────────────────────────

#[test]
fn empty_workbook_produces_a_zip() {
    let wb = Workbook::new();
    let bytes = write_xlsx(&wb);
    assert_eq!(&bytes[..2], b"PK");
}

#[test]
fn workbook_with_empty_sheet_and_no_strings_omits_shared_strings() {
    let mut wb = Workbook::new();
    wb.add_sheet("Empty"); // no cells, no strings
    let bytes = write_xlsx(&wb);
    assert_eq!(&bytes[..2], b"PK");
    // No sharedStrings part should be referenced when there are no text cells.
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("sharedStrings.xml"));
}

// ── Unicode text survives ─────────────────────────────────────────────────

#[test]
fn unicode_text_is_preserved() {
    let mut sst = SharedStrings::default();
    sst.intern("日本語 résumé 🎉");
    let xml = String::from_utf8(sst.to_xml()).unwrap();
    assert!(xml.contains("日本語 résumé 🎉"));
}
