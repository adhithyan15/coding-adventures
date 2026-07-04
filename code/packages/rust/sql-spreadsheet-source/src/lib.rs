//! # `sql-spreadsheet-source` — query a spreadsheet with SQL
//!
//! This crate lets you run SQL over a live [`spreadsheet_core::Workbook`]: each
//! **sheet becomes a table**, the sheet's **first populated row is the column
//! header**, and the rows beneath it are the data. It is a thin
//! [`DataSource`](coding_adventures_sql_execution_engine::DataSource) adapter —
//! all the actual SQL (SELECT / WHERE / GROUP BY / ORDER BY / JOIN / aggregates)
//! is the existing [`sql-execution-engine`](coding_adventures_sql_execution_engine).
//! See `code/specs/SSQL01-sql-spreadsheet-source.md`.
//!
//! ```text
//!   a Workbook (from a .xlsx/.xls/CSV, or edited live in VisiCalc)
//!        │   each sheet = a table
//!        ▼
//!   SpreadsheetSource  ──impl DataSource──▶  sql-execution-engine::execute
//!        │                                            │
//!        └── header row → columns; data rows → rows   └── SELECT … FROM 'Sheet1' WHERE …
//! ```
//!
//! ## Example
//!
//! ```
//! use spreadsheet_core::{CellAddress, CellValue, Workbook};
//! use coding_adventures_sql_spreadsheet_source::query;
//!
//! // A sheet laid out as a table: a header row, then data.
//! let mut wb = Workbook::new();
//! let s = wb.add_sheet("people");
//! for (a1, v) in [("A1", "name"), ("B1", "age")] {
//!     wb.set_value(s, CellAddress::parse(a1).unwrap(), CellValue::Text(v.into()));
//! }
//! wb.set_value(s, CellAddress::parse("A2").unwrap(), CellValue::Text("Ada".into()));
//! wb.set_value(s, CellAddress::parse("B2").unwrap(), CellValue::Number(36.0));
//! wb.set_value(s, CellAddress::parse("A3").unwrap(), CellValue::Text("Grace".into()));
//! wb.set_value(s, CellAddress::parse("B3").unwrap(), CellValue::Number(45.0));
//! wb.recalc_all();
//!
//! let result = query(&wb, "SELECT name FROM people WHERE age > 40").unwrap();
//! assert_eq!(result.columns, vec!["name"]);
//! assert_eq!(result.rows.len(), 1); // just Grace
//! ```

#![forbid(unsafe_code)]

use std::collections::HashMap;

use coding_adventures_sql_execution_engine::{
    execute, DataSource, ExecutionError, QueryResult, SqlPrimitive, SqlValue,
};
use spreadsheet_core::{CellAddress, CellValue, SheetId, Workbook};

// ===========================================================================
// Value + header conversion
// ===========================================================================

/// Convert an engine [`CellValue`] to a nullable SQL value.
///
/// - `Empty` and `Error` become SQL `NULL` — a blank or a `#DIV/0!` is "no
///   value" to a query.
/// - A `Number` that is integral (and fits `i64`) becomes `Int`, else `Float`.
///   This mirrors the CSV source's coercion, so `WHERE age = 40` (an integer
///   literal) matches a spreadsheet `40` naturally.
/// - `Boolean` and `Text` map straight across.
fn cell_to_sql(v: CellValue) -> SqlValue {
    match v {
        CellValue::Empty | CellValue::Error(_) => None,
        CellValue::Boolean(b) => Some(SqlPrimitive::Bool(b)),
        CellValue::Text(s) => Some(SqlPrimitive::Text(s)),
        CellValue::Number(n) => {
            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Some(SqlPrimitive::Int(n as i64))
            } else {
                Some(SqlPrimitive::Float(n))
            }
        }
    }
}

/// The column name a header cell contributes. A header is normally text; a
/// numeric or boolean header is stringified so it can still name a column. An
/// empty header (which shouldn't occur, since we only read *populated* header
/// cells) falls back to `col{n}`.
fn header_name(v: CellValue, col: u32) -> String {
    let name = match v {
        CellValue::Text(s) => s.trim().to_string(),
        CellValue::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Boolean(b) => if b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::Empty | CellValue::Error(_) => String::new(),
    };
    if name.is_empty() {
        format!("col{col}")
    } else {
        name
    }
}

/// Make a header list unique: a repeated name gets a `_2`, `_3`, … suffix. The
/// SQL row model is keyed by column name, so duplicates would otherwise collide
/// and silently drop a column's data.
fn dedupe(names: Vec<String>) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    names
        .into_iter()
        .map(|name| {
            let c = counts.entry(name.clone()).or_insert(0);
            *c += 1;
            if *c == 1 {
                name
            } else {
                format!("{name}_{c}")
            }
        })
        .collect()
}

// ===========================================================================
// The DataSource
// ===========================================================================

/// Exposes a [`Workbook`] to the SQL engine: each **sheet is a table** named by
/// the sheet's name.
///
/// Layout convention: the sheet's **first populated row is the header** (column
/// names), and every populated row below it is a data row. Columns are exactly
/// the populated header cells — a data cell in a column with no header is
/// ignored (SQL needs named columns); a data row is any row below the header
/// that has at least one populated cell.
///
/// Borrows the workbook, so it is a zero-copy view — build one per query.
pub struct SpreadsheetSource<'a> {
    wb: &'a Workbook,
}

impl<'a> SpreadsheetSource<'a> {
    /// View `wb`'s sheets as SQL tables.
    pub fn new(wb: &'a Workbook) -> Self {
        Self { wb }
    }

    /// Resolve a table name to a sheet id, or [`ExecutionError::TableNotFound`].
    fn resolve(&self, table_name: &str) -> Result<SheetId, ExecutionError> {
        self.wb
            .sheet_id(table_name)
            .ok_or_else(|| ExecutionError::TableNotFound(table_name.to_string()))
    }

    /// The `(column, name)` pairs and the header row index for `sheet`.
    ///
    /// Built from the header row's **populated** cells only — iterating the
    /// sparse populated set, never the dense used-range rectangle, so a sheet
    /// that spans a huge range but holds few cells costs `O(populated)`, not
    /// `O(area)`. Returns an empty column list (and header row `1`) for an empty
    /// sheet.
    fn columns(&self, sheet: SheetId) -> (Vec<(u32, String)>, u32) {
        let Some(used) = self.wb.used_range(sheet) else {
            return (Vec::new(), 1);
        };
        let header_row = used.min_row;
        // populated_cells is sorted by (row, col); the header cells are those in
        // the header row, already in column order.
        let raw: Vec<(u32, CellValue)> = self
            .wb
            .populated_cells(sheet)
            .into_iter()
            .filter(|addr| addr.row == header_row)
            .map(|addr| {
                let v = self.wb.get_value(sheet, addr).unwrap_or(CellValue::Empty);
                (addr.col, v)
            })
            .collect();
        let names = dedupe(raw.iter().map(|(c, v)| header_name(v.clone(), *c)).collect());
        let cols = raw.into_iter().map(|(c, _)| c).zip(names).collect();
        (cols, header_row)
    }
}

impl DataSource for SpreadsheetSource<'_> {
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError> {
        let sheet = self.resolve(table_name)?;
        let (cols, _) = self.columns(sheet);
        Ok(cols.into_iter().map(|(_, name)| name).collect())
    }

    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        let sheet = self.resolve(table_name)?;
        let (cols, header_row) = self.columns(sheet);

        // The data rows are the distinct populated rows *below* the header —
        // derived sparsely, so empty gaps in the range never materialise.
        let mut data_rows: Vec<u32> = self
            .wb
            .populated_cells(sheet)
            .into_iter()
            .map(|addr| addr.row)
            .filter(|&row| row > header_row)
            .collect();
        data_rows.sort_unstable();
        data_rows.dedup();

        let rows = data_rows
            .into_iter()
            .map(|row| {
                cols.iter()
                    .map(|(col, name)| {
                        let v = self
                            .wb
                            .get_value(sheet, CellAddress::new(row, *col))
                            .unwrap_or(CellValue::Empty);
                        (name.clone(), cell_to_sql(v))
                    })
                    .collect::<HashMap<String, SqlValue>>()
            })
            .collect();
        Ok(rows)
    }
}

// ===========================================================================
// Convenience
// ===========================================================================

/// Run one SQL statement over `wb` and return the result — the common case,
/// equivalent to `execute(sql, &SpreadsheetSource::new(wb))`.
///
/// Table names in the SQL are sheet names (`FROM 'Sheet1'`). See
/// [`coding_adventures_sql_execution_engine::execute`] for the supported SQL.
pub fn query(wb: &Workbook, sql: &str) -> Result<QueryResult, ExecutionError> {
    execute(sql, &SpreadsheetSource::new(wb))
}

#[cfg(test)]
mod tests;
