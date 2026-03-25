# sql_csv_source — Elixir

A thin adapter that connects the `sql_execution_engine` to CSV files on disk.
Each `tablename.csv` file in a given directory becomes a queryable SQL table.

## Where it fits in the stack

```
sql_execution_engine   ← runs SELECT queries
        │
        │  DataSource behaviour
        ▼
  sql_csv_source       ← this package
        │
        │  CsvParser.parse_csv/1
        ▼
    csv_parser         ← reads raw CSV text into list-of-maps
```

The adapter is intentionally thin:
- **CSV parsing** is fully delegated to `csv_parser`.
- **SQL execution** is fully delegated to `sql_execution_engine`.
- This package only handles file I/O and type coercion.

## Type coercion rules

CSV files store everything as text. `CsvDataSource` converts each string value
to a native Elixir type so that the SQL engine can apply typed comparisons
(`salary > 80000`, `active = true`, `dept_id IS NULL`):

| CSV value        | Elixir value | Notes                              |
|------------------|--------------|------------------------------------|
| `""`             | `nil`        | Empty field → SQL NULL             |
| `"true"`         | `true`       | Case-sensitive                     |
| `"false"`        | `false`      | Case-sensitive                     |
| `"42"`           | `42`         | Integer (no decimal point)         |
| `"3.14"`         | `3.14`       | Float (has decimal point)          |
| `"hello"`        | `"hello"`    | Anything else stays a string       |

## Quick start

```elixir
# Point at a directory that contains employees.csv and departments.csv.
source = CodingAdventures.SqlCsvSource.new("path/to/csvdir")

{:ok, result} = CodingAdventures.SqlCsvSource.execute(
  "SELECT e.name, d.name FROM employees AS e INNER JOIN departments AS d ON e.dept_id = d.id",
  source
)

# result.columns => ["e.name", "d.name"]
# result.rows    => [["Alice", "Engineering"], ["Bob", "Marketing"], ...]
```

You can also use the `DataSource` module directly with `SqlExecutionEngine.execute/2`:

```elixir
alias CodingAdventures.SqlCsvSource.CsvDataSource
alias CodingAdventures.SqlExecutionEngine

source = CsvDataSource.new("path/to/csvdir")

{:ok, result} = SqlExecutionEngine.execute(
  "SELECT * FROM employees WHERE salary > 80000",
  source
)
```

## CSV file conventions

- Files must be named `<tablename>.csv` and live in the directory passed to `new/1`.
- The first row must be the header (column names).
- Column order in the header determines the column order for `SELECT *`.

## Error handling

- Querying a table with no corresponding CSV file raises
  `CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError`.
- Parse errors (malformed CSV) propagate as exceptions from `CsvParser`.
- SQL syntax errors return `{:error, message}` from `SqlExecutionEngine.execute/2`.

## Running tests

```bash
mix test
```

Tests use fixture CSV files in `test/fixtures/`.
