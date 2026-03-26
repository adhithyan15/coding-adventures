# sql-csv-source — Go

A thin adapter that connects the `sql-execution-engine` to CSV files on disk.
Each `tablename.csv` file in a given directory becomes a queryable SQL table.

## Where it fits in the stack

```
sql-execution-engine   ← runs SELECT queries
        │
        │  DataSource interface
        ▼
  sql-csv-source       ← this package
        │
        │  csvparser.ParseCSV()
        ▼
    csv-parser         ← reads raw CSV text into []map[string]string
```

The adapter is intentionally thin:
- **CSV parsing** is fully delegated to `csv-parser`.
- **SQL execution** is fully delegated to `sql-execution-engine`.
- This package only handles file I/O, column ordering, and type coercion.

## Type coercion rules

CSV stores everything as text. `CSVDataSource.Scan` converts each string value
to a native Go type so the SQL engine can apply typed comparisons:

| CSV value | Go type    | Value    | Notes                             |
|-----------|------------|----------|-----------------------------------|
| `""`      | `nil`      | `nil`    | Empty field → SQL NULL            |
| `"true"`  | `bool`     | `true`   | Case-sensitive                    |
| `"false"` | `bool`     | `false`  | Case-sensitive                    |
| `"42"`    | `int64`    | `42`     | Must parse completely (no suffix) |
| `"3.14"`  | `float64`  | `3.14`   | Must parse completely             |
| `"hello"` | `string`   | `"hello"`| Fallthrough — stays as string     |

"Parses completely" means `strconv.ParseInt`/`ParseFloat` succeeds with no
unconsumed suffix — `"123abc"` stays a string, not `int64(123)`.

## Quick start

```go
import (
    sqlcsvsource "github.com/adhithyan15/coding-adventures/code/packages/go/sql-csv-source"
    sqlengine "github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine"
)

source := sqlcsvsource.New("path/to/csvdir")

result, err := sqlengine.Execute(
    "SELECT e.name, d.name FROM employees AS e INNER JOIN departments AS d ON e.dept_id = d.id",
    source,
)
if err != nil {
    log.Fatal(err)
}

fmt.Println(result.Columns) // [e.name d.name]
for _, row := range result.Rows {
    fmt.Println(row)
}
```

## Column ordering

`Schema` returns columns in the order they appear in the CSV header line.
Go maps (`map[string]string`) do not preserve insertion order, so `Schema`
reads the raw CSV header directly — parsing the first line and splitting on
commas — rather than deriving order from the map keys returned by `ParseCSV`.

## Error handling

- `Schema` and `Scan` return `*sqlengine.TableNotFoundError` if the file
  `<dir>/<tableName>.csv` does not exist.
- CSV parse errors (e.g. unclosed quoted field) are propagated as-is.
- SQL syntax errors are returned by `sqlengine.Execute` as wrapped errors.

## Running tests

```bash
go test ./... -v -cover
```

Tests use fixture files in `testdata/`.
