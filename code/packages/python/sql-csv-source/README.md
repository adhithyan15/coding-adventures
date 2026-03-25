# sql-csv-source (Python)

A thin adapter that connects the `sql-execution-engine` to CSV files on disk.
Drop a directory of `tablename.csv` files in front of the SQL engine and
query them with plain `SELECT` statements.

## How it fits in the stack

```
csv files on disk
       │
       ▼
 CsvDataSource          ← this package
       │
       ▼
sql-execution-engine    ← runs SELECT queries
       │
       ▼
  QueryResult
```

## Quick start

```python
from sql_csv_source import CsvDataSource, execute_csv

source = CsvDataSource("path/to/csv/dir")

# Schema discovery
print(source.schema("employees"))
# ["id", "name", "dept_id", "salary", "active"]

# Full SQL queries
result = execute_csv("SELECT * FROM employees WHERE active = true", "path/to/csv/dir")
for row in result.rows:
    print(row)
```

## Type coercion

Every CSV field starts as a string. The adapter coerces values to their
most natural Python type before handing them to the engine:

| CSV string       | Python value         |
|------------------|----------------------|
| `""`             | `None` (SQL NULL)    |
| `"true"`         | `True`               |
| `"false"`        | `False`              |
| `"42"`           | `42` (int)           |
| `"3.14"`         | `3.14` (float)       |
| `"hello"`        | `"hello"` (str)      |

## Installation

```bash
pip install coding-adventures-sql-csv-source
```

## Dependencies

- `coding-adventures-csv-parser` — parses the CSV text into row dicts
- `coding-adventures-sql-execution-engine` — executes SELECT queries
- `coding-adventures-sql-parser` — parses SQL text into ASTs
