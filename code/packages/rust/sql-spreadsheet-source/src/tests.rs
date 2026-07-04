//! Tests for querying a spreadsheet with SQL.

use super::{query, SpreadsheetSource};
use coding_adventures_sql_execution_engine::{execute, DataSource, ExecutionError, SqlPrimitive};
use spreadsheet_core::{CellAddress, CellValue, Workbook};

/// Build a `sales` sheet laid out as a table:
///
/// | region | rep   | amount |
/// |--------|-------|--------|
/// | West   | Ada   | 100    |
/// | East   | Grace | 80     |
/// | West   | Lin   | 150    |
/// | East   | Ada   | 120    |
fn sales_workbook() -> Workbook {
    let mut wb = Workbook::new();
    let s = wb.add_sheet("sales");
    let rows = [
        ("region", "rep", None),
        ("West", "Ada", Some(100.0)),
        ("East", "Grace", Some(80.0)),
        ("West", "Lin", Some(150.0)),
        ("East", "Ada", Some(120.0)),
    ];
    for (r, (a, b, amount)) in rows.iter().enumerate() {
        let row = (r + 1) as u32;
        wb.set_value(s, CellAddress::new(row, 1), CellValue::Text((*a).into()));
        wb.set_value(s, CellAddress::new(row, 2), CellValue::Text((*b).into()));
        match amount {
            Some(n) => wb.set_value(s, CellAddress::new(row, 3), CellValue::Number(*n)),
            None => wb.set_value(s, CellAddress::new(row, 3), CellValue::Text("amount".into())),
        }
    }
    wb.recalc_all();
    wb
}

fn int_of(v: &Option<SqlPrimitive>) -> i64 {
    match v {
        Some(SqlPrimitive::Int(i)) => *i,
        other => panic!("expected Int, got {other:?}"),
    }
}
fn text_of(v: &Option<SqlPrimitive>) -> String {
    match v {
        Some(SqlPrimitive::Text(s)) => s.clone(),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn schema_is_the_header_row() {
    let wb = sales_workbook();
    let src = SpreadsheetSource::new(&wb);
    assert_eq!(
        src.schema("sales").unwrap(),
        vec!["region".to_string(), "rep".to_string(), "amount".to_string()]
    );
}

#[test]
fn scan_returns_data_rows_only() {
    let wb = sales_workbook();
    let src = SpreadsheetSource::new(&wb);
    let rows = src.scan("sales").unwrap();
    assert_eq!(rows.len(), 4, "header row is excluded");
    // Values are typed: amount is an integer, region is text.
    let west = rows.iter().find(|r| text_of(&r["rep"]) == "Ada" && text_of(&r["region"]) == "West");
    assert_eq!(int_of(&west.unwrap()["amount"]), 100);
}

#[test]
fn select_star() {
    let result = query(&sales_workbook(), "SELECT * FROM sales").unwrap();
    assert_eq!(result.rows.len(), 4);
    assert!(result.columns.contains(&"region".to_string()));
}

#[test]
fn where_filters_numeric() {
    let result = query(&sales_workbook(), "SELECT rep FROM sales WHERE amount > 100").unwrap();
    // Lin (150) and Ada-East (120).
    let reps: Vec<String> = result.rows.iter().map(|r| text_of(&r["rep"])).collect();
    assert_eq!(result.rows.len(), 2);
    assert!(reps.contains(&"Lin".to_string()));
    assert!(reps.contains(&"Ada".to_string()));
}

#[test]
fn where_filters_text() {
    let result = query(&sales_workbook(), "SELECT amount FROM sales WHERE region = 'West'").unwrap();
    let amounts: Vec<i64> = result.rows.iter().map(|r| int_of(&r["amount"])).collect();
    assert_eq!(amounts.len(), 2);
    assert!(amounts.contains(&100) && amounts.contains(&150));
}

#[test]
fn aggregate_sum_group_by() {
    let result =
        query(&sales_workbook(), "SELECT region, SUM(amount) FROM sales GROUP BY region").unwrap();
    assert_eq!(result.rows.len(), 2); // West, East
    // Find the SUM column (name may be "SUM(amount)"); grab the aggregate value
    // by locating the non-region column.
    let sum_col = result.columns.iter().find(|c| c.as_str() != "region").unwrap().clone();
    for r in &result.rows {
        let region = text_of(&r["region"]);
        let total = match &r[&sum_col] {
            Some(SqlPrimitive::Int(i)) => *i as f64,
            Some(SqlPrimitive::Float(f)) => *f,
            other => panic!("unexpected sum {other:?}"),
        };
        match region.as_str() {
            "West" => assert_eq!(total, 250.0), // 100 + 150
            "East" => assert_eq!(total, 200.0), // 80 + 120
            other => panic!("unexpected region {other}"),
        }
    }
}

#[test]
fn order_by_and_limit() {
    let result =
        query(&sales_workbook(), "SELECT amount FROM sales ORDER BY amount DESC LIMIT 1").unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(int_of(&result.rows[0]["amount"]), 150);
}

#[test]
fn each_sheet_is_a_table() {
    // Two sheets → two independently queryable tables.
    let mut wb = sales_workbook();
    let t = wb.add_sheet("targets");
    wb.set_value(t, CellAddress::new(1, 1), CellValue::Text("region".into()));
    wb.set_value(t, CellAddress::new(1, 2), CellValue::Text("goal".into()));
    wb.set_value(t, CellAddress::new(2, 1), CellValue::Text("West".into()));
    wb.set_value(t, CellAddress::new(2, 2), CellValue::Number(200.0));
    wb.recalc_all();

    let src = SpreadsheetSource::new(&wb);
    assert_eq!(src.schema("targets").unwrap(), vec!["region", "goal"]);
    assert_eq!(src.scan("targets").unwrap().len(), 1);
    // The other table still works.
    assert_eq!(src.scan("sales").unwrap().len(), 4);
}

#[test]
fn unknown_sheet_is_table_not_found() {
    let wb = sales_workbook();
    let src = SpreadsheetSource::new(&wb);
    assert!(matches!(src.schema("nope"), Err(ExecutionError::TableNotFound(_))));
    assert!(matches!(
        execute("SELECT * FROM nope", &src),
        Err(ExecutionError::TableNotFound(_))
    ));
}

#[test]
fn empty_and_blank_cells_are_null() {
    // A data row with a blank cell → that column is SQL NULL.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("t");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Text("a".into()));
    wb.set_value(s, CellAddress::new(1, 2), CellValue::Text("b".into()));
    wb.set_value(s, CellAddress::new(2, 1), CellValue::Text("x".into()));
    // (2,2) left blank
    wb.recalc_all();

    let rows = SpreadsheetSource::new(&wb).scan("t").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["a"], Some(SqlPrimitive::Text("x".into())));
    assert_eq!(rows[0]["b"], None, "blank cell → NULL");
}

#[test]
fn duplicate_headers_are_disambiguated() {
    let mut wb = Workbook::new();
    let s = wb.add_sheet("t");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Text("x".into()));
    wb.set_value(s, CellAddress::new(1, 2), CellValue::Text("x".into())); // duplicate
    wb.set_value(s, CellAddress::new(2, 1), CellValue::Number(1.0));
    wb.set_value(s, CellAddress::new(2, 2), CellValue::Number(2.0));
    wb.recalc_all();

    let src = SpreadsheetSource::new(&wb);
    assert_eq!(src.schema("t").unwrap(), vec!["x", "x_2"]);
    let rows = src.scan("t").unwrap();
    assert_eq!(int_of(&rows[0]["x"]), 1);
    assert_eq!(int_of(&rows[0]["x_2"]), 2);
}

#[test]
fn sparse_far_corner_does_not_blow_up() {
    // Header row + one data row, but a stray cell far away (row 1,000,000).
    // A dense scan would iterate a million rows; the sparse scan must stay fast
    // and return exactly the real data rows.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("t");
    wb.set_value(s, CellAddress::new(1, 1), CellValue::Text("k".into()));
    wb.set_value(s, CellAddress::new(2, 1), CellValue::Number(1.0));
    wb.set_value(s, CellAddress::new(1_000_000, 1), CellValue::Number(9.0));
    wb.recalc_all();

    let rows = SpreadsheetSource::new(&wb).scan("t").unwrap(); // must return promptly
    // Two data rows: the real one (row 2) and the far stray (row 1_000_000).
    assert_eq!(rows.len(), 2);
}

#[test]
fn end_to_end_query_a_real_xlsx_file() {
    // The headline: build a workbook, save it as a real .xlsx, reopen it via
    // spreadsheet-io, and run SQL over the reopened file.
    let bytes = {
        let wb = sales_workbook();
        spreadsheet_io::save_xlsx(&wb)
    };
    let wb = spreadsheet_io::load_xlsx(&bytes).expect("reopen .xlsx");
    let result = query(&wb, "SELECT rep, amount FROM sales WHERE amount >= 120 ORDER BY amount")
        .expect("SQL over the loaded spreadsheet");
    let reps: Vec<String> = result.rows.iter().map(|r| text_of(&r["rep"])).collect();
    assert_eq!(reps, vec!["Ada".to_string(), "Lin".to_string()]); // 120 then 150
}
