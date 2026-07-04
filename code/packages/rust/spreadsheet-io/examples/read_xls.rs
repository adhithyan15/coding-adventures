//! Load a legacy `.xls` from `argv[1]` and print each populated cell as
//! `Sheet!A1 = value`, one per line. Lets an external tool feed us a file *it*
//! authored (e.g. xlwt) and confirm we read it faithfully.
//!
//! ```text
//! cargo run -p spreadsheet-io --example read_xls -- /tmp/in.xls
//! ```

use spreadsheet_core::CellValue;
use spreadsheet_io::load_xls;

fn main() {
    let path = std::env::args().nth(1).expect("usage: read_xls <in.xls>");
    let bytes = std::fs::read(&path).expect("read file");
    let wb = load_xls(&bytes).expect("parse .xls");

    for name in wb.sheet_names() {
        let sid = wb.sheet_id(name).unwrap();
        for addr in wb.populated_cells(sid) {
            let rendered = match wb.get_value(sid, addr).unwrap_or(CellValue::Empty) {
                CellValue::Number(n) => format!("num:{n}"),
                CellValue::Text(s) => format!("txt:{s}"),
                CellValue::Boolean(b) => format!("bool:{b}"),
                CellValue::Error(e) => format!("err:{}", e.display()),
                CellValue::Empty => continue,
            };
            println!("{name}!{} = {rendered}", addr.to_a1());
        }
    }
}
