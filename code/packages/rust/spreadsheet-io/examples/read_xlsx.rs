//! Load a `.xlsx` from `argv[1]` and print each populated cell as
//! `Sheet!A1 = value [formula]`, one per line. Lets an external tool feed us a
//! file *it* authored (e.g. openpyxl) and confirm we read it faithfully.
//!
//! ```text
//! cargo run -p spreadsheet-io --example read_xlsx -- /tmp/in.xlsx
//! ```

use spreadsheet_core::{CellAddress, CellValue};
use spreadsheet_io::load_xlsx;

fn main() {
    let path = std::env::args().nth(1).expect("usage: read_xlsx <in.xlsx>");
    let bytes = std::fs::read(&path).expect("read file");
    let wb = load_xlsx(&bytes).expect("parse .xlsx");

    for name in wb.sheet_names() {
        let sid = wb.sheet_id(name).unwrap();
        if let Some(ur) = wb.used_range(sid) {
            for row in ur.min_row..=ur.max_row {
                for col in ur.min_col..=ur.max_col {
                    let addr = CellAddress::new(row, col);
                    let v = wb.get_value(sid, addr).unwrap_or(CellValue::Empty);
                    if v == CellValue::Empty {
                        continue;
                    }
                    let rendered = match v {
                        CellValue::Number(n) => format!("num:{n}"),
                        CellValue::Text(s) => format!("txt:{s}"),
                        CellValue::Boolean(b) => format!("bool:{b}"),
                        CellValue::Error(e) => format!("err:{}", e.display()),
                        CellValue::Empty => unreachable!(),
                    };
                    let f = if wb.cell_is_formula(sid, addr) {
                        format!(" [={}]", wb.cell_source_text(sid, addr).trim_start_matches('='))
                    } else {
                        String::new()
                    };
                    println!("{name}!{} = {rendered}{f}", addr.to_a1());
                }
            }
        }
    }
}
