# SSQL01 — `sql-spreadsheet-source`: query a spreadsheet with SQL

Lets you run SQL over a live [`spreadsheet_core::Workbook`] — the same model
VisiCalc computes on and that `spreadsheet-io` loads `.xlsx`/`.xls`/(soon
CSV/JSON) into. Each **sheet becomes a table**. It is a thin adapter: all the
SQL is the existing `sql-execution-engine`; this crate only teaches it how to
read a spreadsheet.

```
  a Workbook (a loaded .xlsx/.xls/CSV, or a live VisiCalc doc)
       │  each sheet = a table
       ▼
  SpreadsheetSource : impl DataSource  ──▶  sql-execution-engine::execute
       │                                          │
       header row → columns; data rows → rows     SELECT … FROM 'Sheet1' WHERE …
```

## Public API

```rust
/// Views a workbook's sheets as SQL tables (borrows the workbook).
pub struct SpreadsheetSource<'a> { /* &Workbook */ }
impl<'a> SpreadsheetSource<'a> { pub fn new(wb: &'a Workbook) -> Self; }
impl DataSource for SpreadsheetSource<'_> { /* schema, scan */ }

/// Convenience: run one statement over a workbook.
pub fn query(wb: &Workbook, sql: &str) -> Result<QueryResult, ExecutionError>;
```

`DataSource`, `QueryResult`, `SqlValue`, `ExecutionError` are re-used from
`sql-execution-engine`; the SQL surface (SELECT / WHERE / GROUP BY / HAVING /
ORDER BY / LIMIT / JOIN / COUNT·SUM·AVG·MIN·MAX) is whatever that engine
supports.

## The sheet-as-table convention

- **Table name = sheet name.** `FROM sales` queries the sheet named `sales`; an
  unknown name is `ExecutionError::TableNotFound`.
- **Header = the first populated row.** Its populated cells define the columns,
  left to right. A numeric/boolean header is stringified; duplicate headers get a
  `_2`, `_3`, … suffix (the SQL row model is keyed by column name, so duplicates
  must be disambiguated).
- **Rows = the populated rows below the header.** A data cell under a column with
  no header is ignored (SQL needs named columns).

## Value mapping (`CellValue` → `SqlValue`)

| `CellValue` | `SqlValue` |
|-------------|-----------|
| `Empty`     | `NULL` |
| `Error(_)`  | `NULL` (an error is not a value) |
| `Number(n)` integral, fits i64 | `Int(n)` |
| `Number(n)` otherwise | `Float(n)` |
| `Text(s)`   | `Text(s)` |
| `Boolean(b)`| `Bool(b)` |

Integral numbers become `Int` (not `Float`) so an integer literal in SQL
(`WHERE age = 40`) matches a spreadsheet `40` — mirroring the CSV source's
coercion.

## Resource safety

`schema`/`scan` iterate the sheet's **populated cells sparsely**
(`populated_cells`), never the dense `used_range` rectangle. A sheet with a
header, one data row, and a stray cell at row 1,000,000 has a ~1e6-row used
range but scans in `O(populated + data_rows × columns)`. A test pins this
(`sparse_far_corner_does_not_blow_up`) — the same DoS lesson as the file writers.

## Non-goals

- Writing query results back into a sheet (a future `dataframe`/range writer).
- Type inference beyond per-cell mapping (a column of mixed Int/Float is left
  mixed, exactly as the CSV source leaves it).
- Cross-workbook joins from SQL text alone (a `DataSource` sees one workbook; the
  engine's JOIN across that workbook's sheets works today).
