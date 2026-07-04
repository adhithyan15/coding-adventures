//! Write the round-trip sample workbook to a legacy `.xls` path given as
//! `argv[1]`, so an external tool (xlrd) can independently verify our output is
//! a genuine `.xls`. Not part of the library — a cross-check helper.
//!
//! ```text
//! cargo run -p spreadsheet-io --example gen_xls -- /tmp/out.xls
//! ```

use spreadsheet_core::{CellAddress, CellValue, Workbook};
use spreadsheet_io::save_xls;

fn main() {
    let path = std::env::args().nth(1).expect("usage: gen_xls <out.xls>");

    let mut wb = Workbook::new();
    let s1 = wb.add_sheet("Sheet1");
    wb.set_value(s1, CellAddress::new(1, 1), CellValue::Number(10.0));
    wb.set_value(s1, CellAddress::new(1, 2), CellValue::Number(20.5));
    wb.set_value(s1, CellAddress::new(1, 3), CellValue::Number(0.5));
    wb.set_formula(s1, CellAddress::new(1, 4), "=SUM(A1:C1)").unwrap(); // -> 31 (value written)
    wb.set_value(s1, CellAddress::new(2, 1), CellValue::Text("hello world".into()));
    wb.set_value(s1, CellAddress::new(2, 2), CellValue::Text("R&D dept".into()));
    let s2 = wb.add_sheet("Budget");
    wb.set_value(s2, CellAddress::new(1, 1), CellValue::Number(-3.25));
    wb.set_formula(s2, CellAddress::new(1, 2), "=A1+100").unwrap(); // -> 96.75 (value written)
    wb.recalc_all();

    std::fs::write(&path, save_xls(&wb)).expect("write .xls");
    println!("wrote {path}");
}
