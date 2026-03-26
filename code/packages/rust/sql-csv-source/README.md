# sql-csv-source (Rust)

A thin adapter that connects `coding-adventures-sql-execution-engine` to CSV
files on disk. Each `tablename.csv` in a directory is one queryable table.

## How it fits in the stack

```
csv files on disk
       │
       ▼
 CsvDataSource          ← this crate
       │
       ▼
sql-execution-engine    ← runs SELECT queries
       │
       ▼
  QueryResult
```

## Quick start

```rust
use coding_adventures_sql_csv_source::CsvDataSource;
use coding_adventures_sql_execution_engine::execute;

let source = CsvDataSource::new("path/to/csv/dir");
let result = execute("SELECT name FROM employees WHERE active = true", &source).unwrap();
for row in &result.rows {
    println!("{:?}", row.get("name"));
}
```

## Type coercion

| CSV string | Rust value              |
|------------|-------------------------|
| `""`       | `None` (SQL NULL)       |
| `"true"`   | `Some(Bool(true))`      |
| `"false"`  | `Some(Bool(false))`     |
| `"42"`     | `Some(Int(42))`         |
| `"3.14"`   | `Some(Float(3.14))`     |
| `"hello"`  | `Some(Text("hello"))`   |

## Dependencies

- `coding-adventures-csv-parser` — parses CSV text into row maps
- `coding-adventures-sql-execution-engine` — executes SELECT queries
