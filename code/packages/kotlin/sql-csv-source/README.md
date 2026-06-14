# sql-csv-source (Kotlin)

CSV-backed data source adapter for the Kotlin mini-sqlite execution engine.

The package maps each `*.csv` file in a directory to a SQL table, parses rows
with `coding_adventures_csv_parser`, coerces scalar values into SQL-friendly
Kotlin values, and executes queries through `SqlExecutionEngine`.
