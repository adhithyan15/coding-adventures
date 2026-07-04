# sql-spreadsheet-source

**Query a spreadsheet with SQL.** A `DataSource` that exposes each sheet of a
`spreadsheet-core::Workbook` as a SQL table, so the existing `sql-execution-engine`
can run `SELECT … FROM 'Sheet1' WHERE …` over a loaded `.xlsx`/`.xls`/CSV — or a
live VisiCalc document.

```rust
use coding_adventures_sql_spreadsheet_source::query;

// `wb` is a spreadsheet-core Workbook (e.g. from spreadsheet_io::load_xlsx).
let result = query(&wb, "SELECT region, SUM(amount) FROM sales GROUP BY region")?;
for row in &result.rows { /* HashMap<column, SqlValue> */ }
```

## Convention

- **Sheet = table** (`FROM sales` → the `sales` sheet).
- **First populated row = header** (column names); rows below are data.
- `CellValue → SqlValue`: integral numbers → `Int`, else `Float`; text → `Text`;
  bool → `Bool`; empty/error → `NULL`. Duplicate headers get a `_2` suffix.

## Demo

```
cargo run -p coding-adventures-sql-spreadsheet-source --example query_xlsx -- \
    sales.xlsx "SELECT rep, amount FROM sales WHERE amount > 100 ORDER BY amount DESC"
```

Pure and DoS-safe (iterates populated cells sparsely, never the dense used-range
rectangle). See `code/specs/SSQL01-sql-spreadsheet-source.md`.
