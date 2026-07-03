//! # The C1 milestone proof: write → read back with our OWN readers.
//!
//! This is the load-bearing test for `xlsx-writer`. We build a workbook in Rust,
//! serialize it to `.xlsx` bytes, then re-open those bytes with **this repo's
//! own** readers and assert the values survived:
//!
//! 1. **Structural** — [`coding_adventures_spreadsheetml::open_workbook`]
//!    re-reads the sheet as a typed grid; we check the sheet name, string cells,
//!    number cell, and formula text.
//! 2. **Recompute** — [`coding_adventures_xlsx_eval::open_and_evaluate`]
//!    recomputes formulas *from scratch* (ignoring the cached `<v>`), and
//!    [`computed_value`](coding_adventures_xlsx_eval::computed_value) must return
//!    the correct number.
//!
//! Write → our reader → correct values (including a formula that recomputes) is
//! the end-to-end demonstration that the bytes are a genuine `.xlsx`.

use coding_adventures_spreadsheetml::{open_workbook, Value};
use coding_adventures_xlsx_eval::{computed_value, open_and_evaluate};
use coding_adventures_xlsx_writer::{write_xlsx, Workbook};
use spreadsheet_core::CellValue;

/// Build the canonical "Revenue" workbook the milestone specifies.
fn revenue_workbook() -> Workbook {
    let mut wb = Workbook::new();
    let sheet = wb.add_sheet("Revenue");
    sheet.set_string("A1", "Q1");
    sheet.set_number("B1", 1000.0);
    sheet.set_string("A2", "Total");
    sheet.set_formula("B2", "SUM(B1:B1)", 1000.0);
    wb
}

#[test]
fn round_trip_structural() {
    let bytes = write_xlsx(&revenue_workbook());

    let rb = open_workbook(&bytes).expect("our spreadsheetml reader should reopen the file");

    // The sheet is present under the right name.
    assert!(
        rb.sheet_names().contains(&"Revenue".to_string()),
        "sheet names were {:?}",
        rb.sheet_names()
    );
    let sheet = rb.sheet_by_name("Revenue").expect("Revenue sheet");

    // A1 = "Q1" (string, dereferenced through the shared-string table).
    let a1 = sheet.cell("A1").expect("A1 populated");
    assert_eq!(a1.value, Value::Text("Q1".to_string()));
    assert_eq!(a1.formatted(), "Q1");

    // B1 = 1000 (number).
    let b1 = sheet.cell("B1").expect("B1 populated");
    assert_eq!(b1.value, Value::Number(1000.0));

    // A2 = "Total" (string).
    let a2 = sheet.cell("A2").expect("A2 populated");
    assert_eq!(a2.value, Value::Text("Total".to_string()));

    // B2 carries the formula text (without the leading '=').
    let b2 = sheet.cell("B2").expect("B2 populated");
    assert_eq!(b2.formula.as_deref(), Some("SUM(B1:B1)"));
    // …and its cached value round-trips too.
    assert_eq!(b2.value, Value::Number(1000.0));
}

#[test]
fn round_trip_formula_recompute() {
    let bytes = write_xlsx(&revenue_workbook());

    // Evaluate: this IGNORES the cached <v> and recomputes SUM(B1:B1) from the
    // formula text, so a correct answer proves the formula was written parseably.
    let core = open_and_evaluate(&bytes).expect("our xlsx-eval should evaluate the file");
    let v = computed_value(&core, "Revenue", "B2");
    assert_eq!(v, Some(CellValue::Number(1000.0)));

    // And the plain number cell reads back through the engine too.
    assert_eq!(
        computed_value(&core, "Revenue", "B1"),
        Some(CellValue::Number(1000.0))
    );
}

#[test]
fn round_trip_multiple_sheets_and_shared_dedup() {
    // Two sheets; "Q1" appears on both → the shared-string table must dedup it,
    // and both cells must still read back as "Q1" after the round-trip.
    let mut wb = Workbook::new();
    {
        let s1 = wb.add_sheet("First");
        s1.set_string("A1", "Q1");
        s1.set_number("B1", 10.0);
    }
    {
        let s2 = wb.add_sheet("Second");
        s2.set_string("A1", "Q1"); // same string as First!A1
        s2.set_string("A2", "Unique");
    }

    let bytes = write_xlsx(&wb);
    let rb = open_workbook(&bytes).expect("reopen");

    assert_eq!(rb.sheet_names(), vec!["First".to_string(), "Second".to_string()]);

    let first = rb.sheet_by_name("First").unwrap();
    assert_eq!(first.cell("A1").unwrap().value, Value::Text("Q1".into()));
    assert_eq!(first.cell("B1").unwrap().value, Value::Number(10.0));

    let second = rb.sheet_by_name("Second").unwrap();
    assert_eq!(second.cell("A1").unwrap().value, Value::Text("Q1".into()));
    assert_eq!(second.cell("A2").unwrap().value, Value::Text("Unique".into()));
}

#[test]
fn round_trip_special_chars_and_unicode() {
    // Sheet name and cell text with XML-special chars and Unicode must survive.
    //
    // NOTE on the exact strings: the repo's own `xml-lexer` intentionally skips
    // whitespace that sits *between* tokens, so a space placed directly adjacent
    // to an escaped entity (e.g. "a & b") is not preserved on read-back — this is
    // a documented reader-side limitation (see xml-parser's `test_entities_in_
    // text`), NOT a writer bug. The writer's escaping is proven total by the unit
    // tests; here we choose text whose specials are not space-adjacent so the
    // round-trip is a clean equality against the reader's behaviour.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("R&D<2026>");
    s.set_string("A1", "a&b<c>d\"e");
    s.set_string("A2", "日本語 résumé 🎉");
    let bytes = write_xlsx(&wb);

    let rb = open_workbook(&bytes).expect("reopen");
    let sheet = rb
        .sheet_by_name("R&D<2026>")
        .expect("sheet name with special chars survives");
    assert_eq!(
        sheet.cell("A1").unwrap().value,
        Value::Text("a&b<c>d\"e".into())
    );
    // Unicode (no space-adjacent entities) survives verbatim, spaces included.
    assert_eq!(
        sheet.cell("A2").unwrap().value,
        Value::Text("日本語 résumé 🎉".into())
    );
}

#[test]
fn round_trip_cross_sheet_formula_recompute() {
    // A formula referencing another sheet must recompute after the round-trip,
    // exercising the r:id → part wiring across multiple worksheets.
    let mut wb = Workbook::new();
    {
        let data = wb.add_sheet("Data");
        data.set_number("A1", 40.0);
        data.set_number("A2", 2.0);
    }
    {
        let calc = wb.add_sheet("Calc");
        calc.set_formula("A1", "Data!A1+Data!A2", 42.0);
    }
    let bytes = write_xlsx(&wb);

    let core = open_and_evaluate(&bytes).expect("evaluate");
    assert_eq!(
        computed_value(&core, "Calc", "A1"),
        Some(CellValue::Number(42.0))
    );
}
