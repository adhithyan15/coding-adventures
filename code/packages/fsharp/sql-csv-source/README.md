# sql-csv-source (F#)

CSV-backed data source adapter for the F# mini-sqlite execution engine.

The package maps each `*.csv` file in a directory to a SQL table, parses rows
with `CodingAdventures.CsvParser`, coerces scalar values into SQL-friendly F#
values, and executes queries through `CodingAdventures.SqlExecutionEngine`.

## Example

```fsharp
open CodingAdventures.SqlCsvSource.FSharp

let result =
    SqlCsvSource.executeCsv
        "SELECT name FROM employees WHERE active = true ORDER BY name"
        "data"
```

## Development

```bash
bash BUILD
```
