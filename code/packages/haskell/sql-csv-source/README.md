# sql-csv-source (Haskell)

A filesystem adapter for the Haskell `sql-execution-engine`. Each `name.csv`
file in a selected directory becomes a queryable SQL table named `name`.

```haskell
import SqlCsvSource

main = do
  result <- executeCsv
    "SELECT name FROM employees WHERE active = true"
    "data"
  print result
```

`loadCsvDataSource` is the explicit IO boundary: it lists and reads the CSV
files once, validates their records, coerces values into `SqlValue`, and
returns an immutable snapshot. `csvDataSource` then exposes that snapshot
through the execution engine's pure `DataSource` callbacks.

The package handles quoted commas, escaped quotes, embedded newlines, CRLF,
header ordering, and row-width validation. Coercion follows the shared lane
contract: empty fields become SQL `NULL`, booleans are case-insensitive,
integers and finite real numbers become numeric values, and all remaining
fields stay text.

## Development

```sh
cabal test all --enable-coverage
```
