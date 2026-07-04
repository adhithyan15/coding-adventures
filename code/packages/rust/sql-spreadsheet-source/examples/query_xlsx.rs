//! Run a SQL query over a real `.xlsx` file:
//!
//! ```text
//! cargo run -p coding-adventures-sql-spreadsheet-source --example query_xlsx -- \
//!     data.xlsx "SELECT region, SUM(amount) FROM sales GROUP BY region"
//! ```
//!
//! Loads the workbook via `spreadsheet-io`, exposes each sheet as a SQL table,
//! runs the query, and prints the result as a simple table. A demonstration that
//! a spreadsheet on disk is directly queryable with SQL.

use coding_adventures_sql_execution_engine::SqlPrimitive;
use coding_adventures_sql_spreadsheet_source::query;

fn cell(v: &Option<SqlPrimitive>) -> String {
    match v {
        None => "NULL".to_string(),
        Some(SqlPrimitive::Int(i)) => i.to_string(),
        Some(SqlPrimitive::Float(f)) => f.to_string(),
        Some(SqlPrimitive::Text(s)) => s.clone(),
        Some(SqlPrimitive::Bool(b)) => b.to_string(),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: query_xlsx <file.xlsx> <SQL>");
    let sql = args.next().expect("usage: query_xlsx <file.xlsx> <SQL>");

    let bytes = std::fs::read(&path).expect("read file");
    let wb = spreadsheet_io::load_xlsx(&bytes).expect("parse .xlsx");
    let result = query(&wb, &sql).expect("run SQL");

    println!("{}", result.columns.join(" | "));
    println!("{}", "-".repeat(result.columns.join(" | ").len().max(3)));
    for row in &result.rows {
        let line: Vec<String> = result.columns.iter().map(|c| cell(&row[c])).collect();
        println!("{}", line.join(" | "));
    }
    println!("({} row{})", result.rows.len(), if result.rows.len() == 1 { "" } else { "s" });
}
