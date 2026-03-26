# sql_csv_source (Ruby)

A thin adapter that connects the `coding_adventures_sql_execution_engine`
to CSV files on disk.  Drop a directory of `tablename.csv` files in front
of the SQL engine and query them with plain `SELECT` statements.

## How it fits in the stack

```
csv files on disk
       │
       ▼
 CsvDataSource          ← this gem
       │
       ▼
sql_execution_engine    ← runs SELECT queries
       │
       ▼
  QueryResult
```

## Quick start

```ruby
require "coding_adventures/sql_csv_source"

source = CodingAdventures::SqlCsvSource::CsvDataSource.new("path/to/csv/dir")

# Schema discovery
source.schema("employees")
# => ["id", "name", "dept_id", "salary", "active"]

# Full SQL queries via the engine
result = CodingAdventures::SqlExecutionEngine.execute(
  "SELECT name FROM employees WHERE active = true",
  source
)
result.rows.each { |row| puts row["name"] }
# Alice
# Bob
# Dave
```

## Type coercion

Every CSV field starts as a string.  The adapter coerces values before
handing them to the engine:

| CSV string  | Ruby value           |
|-------------|----------------------|
| `""`        | `nil` (SQL NULL)     |
| `"true"`    | `true`               |
| `"false"`   | `false`              |
| `"42"`      | `42` (Integer)       |
| `"3.14"`    | `3.14` (Float)       |
| `"hello"`   | `"hello"` (String)   |

## Installation

```bash
gem install coding_adventures_sql_csv_source
```

## Dependencies

- `coding_adventures_csv_parser` — parses CSV text into row hashes
- `coding_adventures_sql_execution_engine` — executes SELECT queries
