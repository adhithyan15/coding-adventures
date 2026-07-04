//! Round-trip and unit tests for the `spreadsheet-io` adapter.
//!
//! The headline property: a workbook built in the engine survives
//! `save_xlsx → load_xlsx` with its values intact and its formulas *still
//! formulas*. We also assert the writer is idempotent (re-saving the reopened
//! workbook is byte-identical) and cover the documented edge cases.

use super::{
    load_csv, load_tsv, load_xls, load_xlsx, save_csv, save_tsv, save_xls, save_xlsx,
};
use spreadsheet_core::{CellAddress, CellValue, SheetId, Workbook};

/// Build the canonical test workbook: two sheets, numbers, text, and a live
/// `=SUM` formula whose result must be preserved.
fn sample_workbook() -> Workbook {
    let mut wb = Workbook::new();

    let s1 = wb.add_sheet("Sheet1");
    wb.set_value(s1, CellAddress::new(1, 1), CellValue::Number(10.0)); // A1
    wb.set_value(s1, CellAddress::new(1, 2), CellValue::Number(20.5)); // B1
    wb.set_value(s1, CellAddress::new(1, 3), CellValue::Number(0.5)); // C1
    wb.set_formula(s1, CellAddress::new(1, 4), "=SUM(A1:C1)").unwrap(); // D1 -> 31.0
    wb.set_value(s1, CellAddress::new(2, 1), CellValue::Text("hello world".into())); // A2
    // B2 exercises XML escaping: the ampersand must survive the writer's
    // `&`→`&amp;` and the reader's `&amp;`→`&`. There is no whitespace *adjacent*
    // to the entity (see `known_limitation_*` below for why that matters).
    wb.set_value(s1, CellAddress::new(2, 2), CellValue::Text("R&D dept".into())); // B2
    wb.set_formula(s1, CellAddress::new(2, 3), "=A1*2").unwrap(); // C2 -> 20.0

    let s2 = wb.add_sheet("Budget");
    wb.set_value(s2, CellAddress::new(1, 1), CellValue::Number(-3.25)); // A1
    wb.set_formula(s2, CellAddress::new(1, 2), "=A1+100").unwrap(); // B1 -> 96.75

    wb.recalc_all();
    wb
}

/// Every populated (sheet, row, col) → its computed value, for equality checks
/// that don't care about internal representation.
fn value_map(wb: &Workbook) -> Vec<(String, u32, u32, CellValue)> {
    let mut out = Vec::new();
    for name in wb.sheet_names() {
        let sid = wb.sheet_id(name).unwrap();
        if let Some(ur) = wb.used_range(sid) {
            for row in ur.min_row..=ur.max_row {
                for col in ur.min_col..=ur.max_col {
                    let addr = CellAddress::new(row, col);
                    let v = wb.get_value(sid, addr).unwrap_or(CellValue::Empty);
                    if v != CellValue::Empty {
                        out.push((name.to_string(), row, col, v));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn output_is_a_zip_package() {
    let bytes = save_xlsx(&sample_workbook());
    assert_eq!(&bytes[..2], b"PK", "an .xlsx is a ZIP, so it starts with PK");
}

#[test]
fn round_trip_preserves_all_values() {
    let wb = sample_workbook();
    let bytes = save_xlsx(&wb);
    let reopened = load_xlsx(&bytes).expect("our own output must reload");

    assert_eq!(
        value_map(&wb),
        value_map(&reopened),
        "every populated cell's computed value must survive save→load"
    );
}

#[test]
fn round_trip_keeps_formulas_as_formulas() {
    let wb = sample_workbook();
    let reopened = load_xlsx(&save_xlsx(&wb)).unwrap();

    let s1 = reopened.sheet_id("Sheet1").unwrap();
    let s2 = reopened.sheet_id("Budget").unwrap();

    // The three formula cells are still formulas (not frozen to numbers)…
    assert!(reopened.cell_is_formula(s1, CellAddress::new(1, 4)), "Sheet1!D1");
    assert!(reopened.cell_is_formula(s1, CellAddress::new(2, 3)), "Sheet1!C2");
    assert!(reopened.cell_is_formula(s2, CellAddress::new(1, 2)), "Budget!B1");

    // …and their cached results are correct.
    assert_eq!(
        reopened.get_value(s1, CellAddress::new(1, 4)),
        Some(CellValue::Number(31.0))
    );
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 2)),
        Some(CellValue::Number(96.75))
    );
}

#[test]
fn reopened_formula_still_recomputes() {
    // A formula that survives as a formula must recompute when its inputs change
    // — proving it's live, not a frozen cache masquerading as a formula.
    let reopened = load_xlsx(&save_xlsx(&sample_workbook())).unwrap();
    let mut wb = reopened;
    let s1 = wb.sheet_id("Sheet1").unwrap();

    // A1 was 10 → D1=SUM(A1:C1)=31. Bump A1 to 110; D1 must become 131.
    wb.set_value(s1, CellAddress::new(1, 1), CellValue::Number(110.0));
    wb.recalc_all();
    assert_eq!(
        wb.get_value(s1, CellAddress::new(1, 4)),
        Some(CellValue::Number(131.0))
    );
}

#[test]
fn save_is_idempotent() {
    // Re-saving a reopened workbook must be byte-identical: the round-trip has a
    // fixed point, so repeated open/save cannot drift the file.
    let once = save_xlsx(&sample_workbook());
    let reopened = load_xlsx(&once).unwrap();
    let twice = save_xlsx(&reopened);
    assert_eq!(once, twice, "save∘load∘save == save");
}

#[test]
fn sheet_names_and_order_survive() {
    let reopened = load_xlsx(&save_xlsx(&sample_workbook())).unwrap();
    assert_eq!(reopened.sheet_names(), vec!["Sheet1", "Budget"]);
}

#[test]
fn text_with_ampersand_round_trips() {
    // "R&D dept" must come back exactly — the writer escapes `&`→`&amp;`, the
    // reader decodes it back, and neither corrupts it.
    let reopened = load_xlsx(&save_xlsx(&sample_workbook())).unwrap();
    let s1 = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(
        reopened.get_value(s1, CellAddress::new(2, 2)),
        Some(CellValue::Text("R&D dept".into()))
    );
}

#[test]
fn known_limitation_whitespace_after_entity_is_dropped() {
    // A documented defect in the shared xml-lexer (NOT this crate): whitespace
    // that sits *immediately after* an entity reference in text content is
    // consumed by the lexer's default-group whitespace skip. So a cell holding
    // "a & b < c" reloads as "a &b <c". We PIN that behaviour here so the round
    // trip is honest about it; when the lexer fix lands (task_4b2c9efe) this
    // test flips to assert exact "a & b < c".
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Text("a & b < c".into()));
    wb.recalc_all();

    let reopened = load_xlsx(&save_xlsx(&wb)).unwrap();
    let s2 = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 1)),
        Some(CellValue::Text("a &b <c".into())),
        "current lexer-limited behaviour; see task_4b2c9efe"
    );
}

#[test]
fn empty_workbook_round_trips() {
    // A workbook with a sheet but no cells is valid and reloads as such.
    let mut wb = Workbook::new();
    wb.add_sheet("Sheet1");
    let reopened = load_xlsx(&save_xlsx(&wb)).unwrap();
    assert_eq!(reopened.sheet_names(), vec!["Sheet1"]);
    let s = reopened.sheet_id("Sheet1").unwrap();
    assert!(reopened.used_range(s).is_none(), "no populated cells");
}

#[test]
fn boolean_literal_is_written_as_number() {
    // Documented limitation: the .xlsx writer has no boolean type, so TRUE is
    // written as 1 and reloads as Number(1). This test *pins* that behaviour so
    // a future fidelity upgrade is a deliberate, visible change.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Boolean(true));
    wb.recalc_all();

    let reopened = load_xlsx(&save_xlsx(&wb)).unwrap();
    let s2 = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 1)),
        Some(CellValue::Number(1.0))
    );
}

#[test]
fn load_rejects_non_xlsx_bytes() {
    // Garbage in → a clean IoError, not a panic. (Workbook isn't Debug, so we
    // match rather than unwrap_err.)
    match load_xlsx(b"not a zip file at all") {
        Ok(_) => panic!("garbage bytes must not parse as .xlsx"),
        Err(e) => {
            assert!(matches!(e, super::IoError::Xlsx(_)));
            assert!(e.to_string().contains("failed to load .xlsx"));
        }
    }
}

#[test]
fn negative_and_fractional_numbers_survive() {
    let reopened = load_xlsx(&save_xlsx(&sample_workbook())).unwrap();
    let s1 = reopened.sheet_id("Sheet1").unwrap();
    let s2 = reopened.sheet_id("Budget").unwrap();
    assert_eq!(
        reopened.get_value(s1, CellAddress::new(1, 2)),
        Some(CellValue::Number(20.5))
    );
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 1)),
        Some(CellValue::Number(-3.25))
    );
}

/// A real `.xlsx` authored by **openpyxl** (a third-party library), checked in
/// as a fixture. It proves we read files other tools produce — the headline
/// capability. Notably openpyxl writes formula cells with an *empty* cached
/// `<v></v>`; the reader must tolerate that (it once errored with `bad number
/// ""`), and our engine then computes the result the producer never did.
const OPENPYXL_AUTHORED: &[u8] = include_bytes!("../tests/fixtures/openpyxl_authored.xlsx");

#[test]
fn reads_a_real_openpyxl_file() {
    let wb = load_xlsx(OPENPYXL_AUTHORED).expect("must open an openpyxl-authored .xlsx");
    assert_eq!(wb.sheet_names(), vec!["Sales", "Notes"]);

    let sales = wb.sheet_id("Sales").unwrap();
    // Literal values openpyxl stored.
    assert_eq!(wb.get_value(sales, CellAddress::new(1, 1)), Some(CellValue::Text("Region".into())));
    assert_eq!(wb.get_value(sales, CellAddress::new(2, 2)), Some(CellValue::Number(100.0)));

    // The two formula cells: openpyxl left them UNcomputed (empty <v>), yet they
    // load as live formulas and our engine fills in the correct sums.
    assert!(wb.cell_is_formula(sales, CellAddress::new(4, 2)), "B4 is a formula");
    assert_eq!(wb.get_value(sales, CellAddress::new(4, 2)), Some(CellValue::Number(180.0)));
    assert_eq!(wb.get_value(sales, CellAddress::new(4, 3)), Some(CellValue::Number(270.0)));

    // Text with an escaped ampersand from the second sheet.
    let notes = wb.sheet_id("Notes").unwrap();
    assert_eq!(
        wb.get_value(notes, CellAddress::new(1, 1)),
        Some(CellValue::Text("R&D budget = 5%".into()))
    );
}

#[test]
fn real_file_re_saves_and_reloads() {
    // Full interop loop: read a foreign file, save it as our .xlsx, reopen —
    // values (including the now-computed formula results) must be stable.
    let wb = load_xlsx(OPENPYXL_AUTHORED).unwrap();
    let reopened = load_xlsx(&save_xlsx(&wb)).unwrap();
    assert_eq!(value_map(&wb), value_map(&reopened));
}

#[test]
fn sparse_far_corner_saves_fast_and_round_trips() {
    // Two cells at opposite corners of the sheet: A1 and XFD1048576 (Excel's max
    // cell). The used range spans ~17 billion positions, but only two cells are
    // populated. save_xlsx must walk the SPARSE set (populated_cells), not the
    // dense rectangle — otherwise this test would hang for hours. If it returns
    // promptly, the DoS is closed.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Number(1.0)); // A1
    wb.set_value(s, CellAddress::new(1_048_576, 16_384), CellValue::Number(2.0)); // XFD1048576
    wb.recalc_all();

    let bytes = save_xlsx(&wb); // must complete quickly (sparse walk)
    let reopened = load_xlsx(&bytes).unwrap();
    let s2 = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(reopened.get_value(s2, CellAddress::new(1, 1)), Some(CellValue::Number(1.0)));
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1_048_576, 16_384)),
        Some(CellValue::Number(2.0))
    );
    // Exactly two populated cells survive — nothing spurious was materialized.
    assert_eq!(reopened.populated_cells(s2).len(), 2);
}

/// A tiny compile-time-ish guard that `SheetId` round-trips through the public
/// accessors we rely on (kept trivial; the real coverage is the round-trips).
#[test]
fn sheet_id_lookup_is_stable() {
    let wb = sample_workbook();
    let a: SheetId = wb.sheet_id("Sheet1").unwrap();
    let b: SheetId = wb.sheet_id("Sheet1").unwrap();
    assert_eq!(a, b);
}

// =========================================================================
// Legacy .xls (BIFF8) — SSIO02
// =========================================================================

#[test]
fn xls_output_is_ole2() {
    // A .xls is an OLE2 compound file: it starts with the D0CF11E0 magic.
    let bytes = save_xls(&sample_workbook());
    assert_eq!(
        &bytes[..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        "an .xls begins with the OLE2 signature"
    );
}

#[test]
fn xls_round_trip_preserves_numbers_and_text() {
    // Numbers and text round-trip exactly through .xls. Formulas do NOT survive
    // as formulas (the .xls writer has no formula record) — their *computed
    // value* is written — so we compare computed values, which must match.
    let wb = sample_workbook();
    let reopened = load_xls(&save_xls(&wb)).expect("our own .xls must reload");
    assert_eq!(
        value_map(&wb),
        value_map(&reopened),
        "every populated cell's computed value must survive .xls save→load"
    );
}

#[test]
fn xls_flattens_formulas_to_values() {
    // Document + pin the .xls fidelity limit: a formula cell comes back as a
    // plain value (the computed result), NOT a formula. Its value is still right.
    let wb = sample_workbook();
    let reopened = load_xls(&save_xls(&wb)).unwrap();
    let s1 = reopened.sheet_id("Sheet1").unwrap();

    assert_eq!(
        reopened.get_value(s1, CellAddress::new(1, 4)), // D1 was =SUM(A1:C1)
        Some(CellValue::Number(31.0)),
        "the formula's computed value is preserved"
    );
    assert!(
        !reopened.cell_is_formula(s1, CellAddress::new(1, 4)),
        ".xls can't store the formula, so it reloads as a literal value"
    );
}

#[test]
fn xls_save_is_idempotent() {
    let once = save_xls(&sample_workbook());
    let reopened = load_xls(&once).unwrap();
    let twice = save_xls(&reopened);
    assert_eq!(once, twice, "save∘load∘save == save for .xls");
}

#[test]
fn xls_sheet_names_survive() {
    let reopened = load_xls(&save_xls(&sample_workbook())).unwrap();
    assert_eq!(reopened.sheet_names(), vec!["Sheet1", "Budget"]);
}

#[test]
fn xls_negative_and_fractional_numbers_survive() {
    let reopened = load_xls(&save_xls(&sample_workbook())).unwrap();
    let s1 = reopened.sheet_id("Sheet1").unwrap();
    let s2 = reopened.sheet_id("Budget").unwrap();
    assert_eq!(
        reopened.get_value(s1, CellAddress::new(1, 2)),
        Some(CellValue::Number(20.5))
    );
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 1)),
        Some(CellValue::Number(-3.25))
    );
}

#[test]
fn xls_error_cell_maps_to_display_text() {
    // The .xls writer has no error record, so an engine error is written as its
    // display text and reloads as that Text. (A .xlsx→.xls degradation path.)
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(
        s,
        CellAddress::new(1, 1),
        CellValue::Error(spreadsheet_core::SpreadsheetError::DivZero),
    );
    wb.recalc_all();

    let reopened = load_xls(&save_xls(&wb)).unwrap();
    let s2 = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(
        reopened.get_value(s2, CellAddress::new(1, 1)),
        Some(CellValue::Text("#DIV/0!".into()))
    );
}

#[test]
fn xls_empty_workbook_round_trips() {
    let mut wb = Workbook::new();
    wb.add_sheet("Sheet1");
    let reopened = load_xls(&save_xls(&wb)).unwrap();
    assert_eq!(reopened.sheet_names(), vec!["Sheet1"]);
    let s = reopened.sheet_id("Sheet1").unwrap();
    assert!(reopened.used_range(s).is_none());
}

#[test]
fn xls_load_rejects_non_xls_bytes() {
    match load_xls(b"PK\x03\x04 this is a zip, not an OLE2 file") {
        Ok(_) => panic!("zip bytes must not parse as .xls"),
        Err(e) => {
            assert!(matches!(e, super::IoError::Xls(_)));
            assert!(e.to_string().contains("failed to load .xls"));
        }
    }
}

/// A real `.xls` authored by **xlwt** (a third-party library), checked in as a
/// fixture — proof we read legacy files other tools produce.
const XLWT_AUTHORED: &[u8] = include_bytes!("../tests/fixtures/xlwt_authored.xls");

#[test]
fn reads_a_real_xlwt_file() {
    let wb = load_xls(XLWT_AUTHORED).expect("must open an xlwt-authored .xls");
    assert_eq!(wb.sheet_names(), vec!["Sales", "Notes"]);

    let sales = wb.sheet_id("Sales").unwrap();
    assert_eq!(wb.get_value(sales, CellAddress::new(1, 1)), Some(CellValue::Text("Region".into())));
    assert_eq!(wb.get_value(sales, CellAddress::new(2, 2)), Some(CellValue::Number(100.0)));
    assert_eq!(wb.get_value(sales, CellAddress::new(3, 3)), Some(CellValue::Number(120.0)));

    let notes = wb.sheet_id("Notes").unwrap();
    assert_eq!(
        wb.get_value(notes, CellAddress::new(1, 1)),
        Some(CellValue::Text("R&D budget = 5%".into()))
    );

    // Documented limitation: xlwt wrote the SUM cells as formulas with no cached
    // value, and the .xls reader decodes cached values but not formula
    // expressions — so B4/C4 carry no recoverable result (no live formula to
    // recompute, unlike the .xlsx path). They are NOT live formulas.
    assert!(!wb.cell_is_formula(sales, CellAddress::new(4, 2)));
}

#[test]
fn biff_error_code_mapping() {
    use spreadsheet_core::SpreadsheetError::*;
    assert_eq!(super::biff_error_to_core(0x07), DivZero);
    assert_eq!(super::biff_error_to_core(0x17), Ref);
    assert_eq!(super::biff_error_to_core(0x1D), Name);
    assert_eq!(super::biff_error_to_core(0x2A), NotAvailable);
    assert_eq!(super::biff_error_to_core(0x00), Null);
    assert_eq!(super::biff_error_to_core(0x0F), Value);
    assert_eq!(super::biff_error_to_core(0x24), Num);
    assert_eq!(super::biff_error_to_core(0xFF), Value); // unknown → generic
}

// =========================================================================
// Delimited text (CSV / TSV) — SSIO-CSV
// =========================================================================

fn cell(wb: &Workbook, a1: &str) -> CellValue {
    let s = wb.sheet_id("Sheet1").unwrap();
    wb.get_value(s, CellAddress::parse(a1).unwrap())
        .unwrap_or(CellValue::Empty)
}

#[test]
fn csv_loads_grid_with_type_coercion() {
    let wb = load_csv(b"name,age,active\nAda,36,yes\nGrace,45,no\n").unwrap();
    assert_eq!(cell(&wb, "A1"), CellValue::Text("name".into())); // header stays text
    assert_eq!(cell(&wb, "A2"), CellValue::Text("Ada".into()));
    assert_eq!(cell(&wb, "B2"), CellValue::Number(36.0)); // numeric field → Number
    assert_eq!(cell(&wb, "C2"), CellValue::Text("yes".into()));
}

#[test]
fn csv_round_trips_numbers_and_text() {
    let original = b"region,amount\nWest,100\nEast,80.5\n";
    let wb = load_csv(original).unwrap();
    let out = save_csv(&wb);
    // Numbers render without trailing .0; text unchanged.
    assert_eq!(out, b"region,amount\nWest,100\nEast,80.5", "{}", String::from_utf8_lossy(&out));
    // And reloading yields the same values.
    let wb2 = load_csv(&out).unwrap();
    assert_eq!(cell(&wb2, "B2"), CellValue::Number(100.0));
    assert_eq!(cell(&wb2, "B3"), CellValue::Number(80.5));
}

#[test]
fn csv_quotes_fields_that_need_it() {
    // A field with a comma, a quote, and a newline must be RFC-4180 quoted.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Text("a,b".into()));
    wb.set_value(s, CellAddress::new(1, 2), CellValue::Text("he said \"hi\"".into()));
    wb.set_value(s, CellAddress::new(2, 1), CellValue::Text("line1\nline2".into()));
    wb.recalc_all();

    let out = String::from_utf8(save_csv(&wb)).unwrap();
    assert_eq!(out, "\"a,b\",\"he said \"\"hi\"\"\"\n\"line1\nline2\",");
    // Round-trips: the quoting is understood on the way back in.
    let wb2 = load_csv(out.as_bytes()).unwrap();
    assert_eq!(cell(&wb2, "A1"), CellValue::Text("a,b".into()));
    assert_eq!(cell(&wb2, "B1"), CellValue::Text("he said \"hi\"".into()));
    assert_eq!(cell(&wb2, "A2"), CellValue::Text("line1\nline2".into()));
}

#[test]
fn tsv_uses_tabs() {
    let wb = load_tsv(b"a\tb\n1\t2\n").unwrap();
    assert_eq!(cell(&wb, "A2"), CellValue::Number(1.0));
    assert_eq!(cell(&wb, "B2"), CellValue::Number(2.0));
    let out = save_tsv(&wb);
    assert_eq!(out, b"a\tb\n1\t2");
}

#[test]
fn csv_formula_saves_as_computed_value() {
    // A CSV has no formulas: the computed value is written.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("Sheet1");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Number(10.0));
    wb.set_value(s, CellAddress::new(1, 2), CellValue::Number(20.0));
    wb.set_formula(s, CellAddress::new(1, 3), "=A1+B1").unwrap();
    wb.recalc_all();
    assert_eq!(save_csv(&wb), b"10,20,30");
}

#[test]
fn csv_save_writes_only_first_sheet() {
    let mut wb = Workbook::new();
    let s1 = wb.add_sheet("Sheet1");
    wb.set_value(s1, CellAddress::new(1, 1), CellValue::Text("first".into()));
    let s2 = wb.add_sheet("Sheet2");
    wb.set_value(s2, CellAddress::new(1, 1), CellValue::Text("second".into()));
    wb.recalc_all();
    assert_eq!(save_csv(&wb), b"first");
}

#[test]
fn csv_load_rejects_invalid_utf8() {
    match load_csv(&[0xff, 0xfe, 0x00]) {
        Ok(_) => panic!("invalid UTF-8 must not load as CSV"),
        Err(e) => {
            assert!(matches!(e, super::IoError::Csv(_)));
            assert!(e.to_string().contains("failed to load CSV/TSV"));
        }
    }
}

#[test]
fn csv_empty_input_is_empty_workbook() {
    let wb = load_csv(b"").unwrap();
    let s = wb.sheet_id("Sheet1").unwrap();
    assert!(wb.used_range(s).is_none());
    assert_eq!(save_csv(&wb), b"");
}

#[test]
fn csv_to_xlsx_bridge() {
    // A CSV loaded into the engine can be saved right back out as a real .xlsx —
    // the whole point of a single hub: any format in, any format out.
    let wb = load_csv(b"h1,h2\n1,two\n").unwrap();
    let xlsx = save_xlsx(&wb);
    assert_eq!(&xlsx[..2], b"PK");
    let reopened = load_xlsx(&xlsx).unwrap();
    let s = reopened.sheet_id("Sheet1").unwrap();
    assert_eq!(reopened.get_value(s, CellAddress::new(2, 1)), Some(CellValue::Number(1.0)));
    assert_eq!(reopened.get_value(s, CellAddress::new(2, 2)), Some(CellValue::Text("two".into())));
}
