//! Write the round-trip sample workbook to a `.xlsx` path given as `argv[1]`,
//! so an external tool (openpyxl) can independently verify our output is a
//! genuine `.xlsx`. Not part of the library — a cross-check helper.
//!
//! ```text
//! cargo run -p spreadsheet-io --example gen_xlsx -- /tmp/out.xlsx
//! ```

use spreadsheet_core::{CellAddress, CellValue, Workbook};
use spreadsheet_io::save_xlsx;

fn main() {
    let path = std::env::args().nth(1).expect("usage: gen_xlsx <out.xlsx>");

    let mut wb = Workbook::new();
    let s1 = wb.add_sheet("Sheet1");
    wb.set_value(s1, CellAddress::new(1, 1), CellValue::Number(10.0));
    wb.set_value(s1, CellAddress::new(1, 2), CellValue::Number(20.5));
    wb.set_value(s1, CellAddress::new(1, 3), CellValue::Number(0.5));
    wb.set_formula(s1, CellAddress::new(1, 4), "=SUM(A1:C1)").unwrap();
    wb.set_value(s1, CellAddress::new(2, 1), CellValue::Text("hello world".into()));
    wb.set_value(s1, CellAddress::new(2, 2), CellValue::Text("R&D dept".into()));
    let s2 = wb.add_sheet("Budget");
    wb.set_value(s2, CellAddress::new(1, 1), CellValue::Number(-3.25));
    wb.set_formula(s2, CellAddress::new(1, 2), "=A1+100").unwrap();
    wb.recalc_all();

    std::fs::write(&path, save_xlsx(&wb)).expect("write .xlsx");
    println!("wrote {path}");
}
